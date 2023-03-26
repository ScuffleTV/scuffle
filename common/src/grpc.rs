use std::{collections::HashSet, fmt, net::SocketAddr, time::Duration};

use anyhow::Result;
use async_trait::async_trait;
use futures::future;
use http::Uri;
use tokio::sync::mpsc::Sender;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint, Identity};
use tower::discover::Change;
use trust_dns_resolver::{
    error::ResolveError,
    lookup::Lookup,
    proto::rr::{RData, RecordType},
    TokioAsyncResolver,
};

#[derive(Clone, Debug)]
/// Options for creating a gRPC channel.
pub struct ChannelOpts {
    /// A list of addresses to connect to. If this is empty, will return an error.
    /// Can be a hostname or an IP address.
    pub addresses: Vec<String>,
    /// If true, will try to resolve CNAME records for the hostname.
    /// Useful for headless services. If false, will only resolve A/AAAA records.
    pub try_cname: bool,
    /// If true, will try to resolve IPv6 addresses.
    pub enable_ipv6: bool,
    /// If true, will try to resolve IPv4 addresses.
    pub enable_ipv4: bool,
    /// How often to re-resolve the hostnames. If this is 0, will only resolve once.
    pub interval: Duration,
    /// TLS settings. Is None if TLS is disabled. If this is Some, will use TLS.
    pub tls: Option<TlsSettings>,
}

#[derive(Clone, Debug)]
pub struct TlsSettings {
    /// The domain on the certificate.
    pub domain: String,
    /// The client certificate.
    pub identity: Identity,
    /// The CA certificate to verify the server.
    pub ca_cert: Certificate,
}

/// Internal struct for controlling the channel.
/// Automatically resolves hostnames and handles DNS changes.
struct ChannelController<R: DnsResolver> {
    last_addresses: HashSet<EndpointType>,
    resolver: R,
    try_cname: bool,
    enable_ipv6: bool,
    enable_ipv4: bool,
    sender: Sender<Change<EndpointType, Endpoint>>,
    interval: Duration,
    hostnames: HashSet<(String, u16)>,
    static_ips: HashSet<SocketAddr>,
    tls: Option<TlsSettings>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// A wrapper around SocketAddr and CNAME records.
/// Hashable to be used in a HashSet.
enum EndpointType {
    Ip(SocketAddr),
    CName(String, u16),
}

impl fmt::Display for EndpointType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ip(addr) => write!(f, "{}", addr),
            Self::CName(name, port) => write!(f, "{}:{}", name, port),
        }
    }
}

#[async_trait]
pub trait DnsResolver: Send + Sync + 'static {
    async fn lookup(&self, hostname: &str, record_type: RecordType)
        -> Result<Lookup, ResolveError>;
}

#[async_trait]
impl DnsResolver for TokioAsyncResolver {
    #[inline(always)]
    async fn lookup(
        &self,
        hostname: &str,
        record_type: RecordType,
    ) -> Result<Lookup, ResolveError> {
        self.lookup(hostname, record_type).await
    }
}

struct ChannelControllerOpts {
    sender: Sender<Change<EndpointType, Endpoint>>,
    interval: Duration,
    hostnames: HashSet<(String, u16)>,
    static_ips: HashSet<SocketAddr>,
    tls: Option<TlsSettings>,
    try_cname: bool,
    enable_ipv6: bool,
    enable_ipv4: bool,
}

impl<R: DnsResolver> ChannelController<R> {
    fn new(resolver: R, opts: ChannelControllerOpts) -> Result<Self> {
        Ok(Self {
            last_addresses: HashSet::new(),
            resolver,
            sender: opts.sender,
            interval: opts.interval,
            hostnames: opts.hostnames,
            static_ips: opts.static_ips,
            try_cname: opts.try_cname,
            enable_ipv6: opts.enable_ipv6,
            enable_ipv4: opts.enable_ipv4,
            tls: opts.tls,
        })
    }

    /// Starts the controller
    pub async fn start(mut self) {
        // We start by running the first lookup.
        while self.run().await {
            // if the interval is 0, we only run once.
            if self.interval == Duration::from_secs(0) {
                break;
            }

            tokio::time::sleep(self.interval).await;
        }
    }

    /// Runs a single lookup.
    async fn run(&mut self) -> bool {
        let mut addresses = self
            .static_ips
            .clone()
            .into_iter()
            .map(EndpointType::Ip)
            .collect::<HashSet<_>>();

        let futures = self.hostnames.iter().map(|(hostname, port)| {
            let resolver = &self.resolver;
            let try_cname = self.try_cname;
            let port = *port;

            let enable_ipv4 = self.enable_ipv4;
            let enable_ipv6 = self.enable_ipv6;

            // This needs to be a move, because we need the port and hostname.
            async move {
                if try_cname {
                    let cname = resolver.lookup(hostname, RecordType::CNAME).await;
                    if let Ok(cname) = cname {
                        return Ok((cname, port));
                    }
                }

                if enable_ipv4 {
                    let lookup = resolver.lookup(hostname, RecordType::A).await;
                    if let Ok(lookup) = lookup {
                        return Ok((lookup, port));
                    }
                }

                if enable_ipv6 {
                    let lookup = resolver.lookup(hostname, RecordType::AAAA).await;
                    if let Ok(lookup) = lookup {
                        return Ok((lookup, port));
                    }
                }

                Err(anyhow::anyhow!("Failed to resolve hostname: {}", hostname))
            }
        });

        future::join_all(futures)
            .await
            .into_iter()
            .for_each(|result| match result {
                // If the lookup was successful, we add all the addresses to the total list.
                Ok((lookup, port)) => {
                    lookup
                        .into_iter()
                        // We convert the IpAddr to a SocketAddr, so we can add it to the HashSet.
                        // Since we are using a filter_map here, we can also filter out any records that we don't care about.
                        .filter_map(move |record| {
                            match record {
                                // If we get an A record back, we convert it to an SocketAddr with the port and then into a EndpointType::Ip.
                                RData::A(a) => {
                                    Some(EndpointType::Ip(SocketAddr::new(a.into(), port)))
                                }
                                // If we get an AAAA record back, we convert it to an SocketAddr with the port and then into a EndpointType::Ip.
                                RData::AAAA(aaaa) => {
                                    Some(EndpointType::Ip(SocketAddr::new(aaaa.into(), port)))
                                }
                                // If we get a CNAME record back, we convert it to an EndpointType::CName with the port.
                                RData::CNAME(cname) => {
                                    Some(EndpointType::CName(cname.to_string(), port))
                                }
                                // This should never happen, but we have to handle it. We just ignore it.
                                _ => None,
                            }
                        })
                        // Now for all the records we got back, we add them to the HashSet.
                        .for_each(|endpoint| {
                            // This is a HashSet, so we don't have to worry about duplicates.
                            addresses.insert(endpoint);
                        });
                }
                Err(e) => {
                    // If the lookup failed, we log the error.
                    tracing::debug!("failed to lookup address: {}", e);
                }
            });

        // Now we check if there are any new addresses.
        let added = addresses
            .difference(&self.last_addresses)
            // If we have a new address, we need to construct a Change to add it to the channel.
            .filter_map(|addr| {
                // First we need to make a Endpoint from the EndpointType.
                let endpoint = if self.tls.is_some() {
                    Endpoint::from_shared(format!("https://{}", addr))
                } else {
                    Endpoint::from_shared(format!("http://{}", addr))
                };

                // If we failed to make a Endpoint, we log the error and return None.
                let endpoint = match endpoint {
                    Ok(endpoint) => endpoint,
                    Err(e) => {
                        tracing::warn!("invalid address: {}, {}", addr, e);
                        return None;
                    }
                };

                // If TLS is enabled, we need to add the TLS config to the Endpoint.
                let endpoint = if self.tls.is_some() {
                    let tls = self.tls.as_ref().unwrap();
                    let tls = ClientTlsConfig::new()
                        .domain_name(tls.domain.clone())
                        .ca_certificate(tls.ca_cert.clone())
                        .identity(tls.identity.clone());

                    match endpoint.tls_config(tls) {
                        Ok(endpoint) => endpoint,
                        Err(e) => {
                            tracing::warn!("invalid tls config: {}: {}", addr, e);
                            return None;
                        }
                    }
                } else {
                    endpoint
                };

                // We now construct the Change and return it.
                Some(Change::Insert(addr.clone(), endpoint))
            });

        // Now we check if there are any addresses that have been removed.
        let removed = self
            .last_addresses
            .difference(&addresses)
            // We construct a Change to remove the address from the channel.
            .map(|addr| Change::Remove(addr.clone()));

        // We combine the 2 streams into one.
        let changes = added.chain(removed);

        // Now we send all the changes to the channel.
        for change in changes {
            // If this fails, it means the receiver has been dropped, so we can stop the loop.
            if self.sender.send(change).await.is_err() {
                tracing::debug!("channel controller stopped");
                return false;
            }
        }

        // We then update the last_addresses HashSet with the new addresses.
        self.last_addresses = addresses;

        // We return true, so the loop will continue.
        true
    }
}

/// Make a new gRPC transport channel which is backed by a DNS resolver.
/// This will create a new channel which will automatically update the endpoints
/// when the DNS records change. Allowing for a more dynamic way of connecting
/// to services.
#[inline(always)]
pub fn make_channel(
    addresses: Vec<String>,
    interval: Duration,
    tls: Option<TlsSettings>,
) -> Result<Channel> {
    make_channel_with_opts(ChannelOpts {
        addresses,
        tls,
        interval,
        enable_ipv4: true,
        enable_ipv6: true,
        try_cname: true,
    })
}

/// Make a new gRPC transport channel which is backed by a DNS resolver.
/// This will create a new channel which will automatically update the endpoints
/// when the DNS records change. Allowing for a more dynamic way of connecting
/// to services. This funtion allows you to provide your own options.
#[inline(always)]
pub fn make_channel_with_opts(opts: ChannelOpts) -> Result<Channel> {
    make_channel_with_resolver(TokioAsyncResolver::tokio_from_system_conf()?, opts)
}

/// Make a new gRPC transport channel which is backed by a DNS resolver.
/// This will create a new channel which will automatically update the endpoints
/// when the DNS records change. Allowing for a more dynamic way of connecting
/// to services. This function allows you to provide your own DNS resolver.
/// This is useful if you want to use a different DNS resolver, or if you want
/// to unit test this function.
pub fn make_channel_with_resolver<R: DnsResolver>(
    resolver: R,
    opts: ChannelOpts,
) -> Result<Channel> {
    // We first check if any addresses were provided.
    if opts.addresses.is_empty() {
        return Err(anyhow::anyhow!("no addresses provided"));
    }

    // 128 is an arbitrary number, but it should be enough for most use cases.
    let (channel, sender) = Channel::balance_channel(128);

    let mut static_ips = HashSet::new();
    let mut hostnames = HashSet::new();

    // We iterate over the provided addresses and parse them into a Uri.
    // So we can check if the address is a hostname or an IP address.
    for address in opts.addresses {
        let uri = address.parse::<Uri>()?;

        // Get the port from the Uri, or use the default port.
        let port = uri
            .port_u16()
            .unwrap_or(if opts.tls.is_some() { 443 } else { 80 });

        // Get the host from the Uri, or return an error if it doesn't exist.
        let Some(address) = uri.host() else {
            return Err(anyhow::anyhow!("invalid address: {}", address));
        };

        // If the address is an IP address, we add it to the ip_addresses HashSet.
        if let Ok(addr) = address.parse::<std::net::IpAddr>() {
            static_ips.insert(SocketAddr::new(addr, port));
        } else {
            hostnames.insert((address.to_string(), port));
        }
    }

    // We now create a new ChannelController
    // The channel controller will handle the DNS lookups and updating the channel.
    let controller = ChannelController::new(
        resolver,
        ChannelControllerOpts {
            sender,
            interval: opts.interval,
            hostnames,
            static_ips,
            tls: opts.tls,
            try_cname: opts.try_cname,
            enable_ipv6: opts.enable_ipv6,
            enable_ipv4: opts.enable_ipv4,
        },
    )?;

    // We spawn the controller on a new task.
    tokio::spawn(controller.start());

    // We return the channel.
    // The channel will be updated by the controller.
    Ok(channel)
}
