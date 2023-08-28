use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use tokio::sync::Mutex;
use tonic::transport::Server;
use trust_dns_resolver::{
    error::ResolveError,
    lookup::Lookup,
    proto::{
        op::Query,
        rr::{
            rdata::{A, AAAA, CNAME},
            RData, Record, RecordType,
        },
    },
    Name,
};

use crate::grpc::{
    make_channel, make_channel_with_opts, make_channel_with_resolver, ChannelOpts, DnsResolver,
};

mod pb {
    tonic::include_proto!("test");
}

struct TestImpl {
    name: String,
}

#[async_trait]
impl pb::test_server::Test for TestImpl {
    async fn test(
        &self,
        request: tonic::Request<pb::TestRequest>,
    ) -> Result<tonic::Response<pb::TestResponse>, tonic::Status> {
        Ok(tonic::Response::new(pb::TestResponse {
            message: request.into_inner().message,
            server: self.name.clone(),
        }))
    }
}

#[tokio::test]
async fn test_static_ip_resolve() {
    let addr_1 = SocketAddr::from((
        [127, 0, 0, 1],
        portpicker::pick_unused_port().expect("failed to pick port"),
    ));
    tokio::spawn(
        Server::builder()
            .add_service(pb::test_server::TestServer::new(TestImpl {
                name: "server1".to_string(),
            }))
            .serve(addr_1),
    );

    let addr_2 = SocketAddr::from((
        [127, 0, 0, 1],
        portpicker::pick_unused_port().expect("failed to pick port"),
    ));
    tokio::spawn(
        Server::builder()
            .add_service(pb::test_server::TestServer::new(TestImpl {
                name: "server2".to_string(),
            }))
            .serve(addr_2),
    );

    let channel = make_channel(
        vec![addr_1.to_string(), addr_2.to_string()],
        Duration::from_secs(0),
        None,
    )
    .unwrap();
    let mut client = pb::test_client::TestClient::new(channel);

    let mut server_1 = 0;
    let mut server_2 = 0;

    const NUM_REQUESTS: usize = 1000;

    for _ in 0..NUM_REQUESTS {
        let response = client
            .test(tonic::Request::new(pb::TestRequest {
                message: "test".to_string(),
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.message, "test");

        if response.server == "server1" {
            server_1 += 1;
        } else if response.server == "server2" {
            server_2 += 1;
        } else {
            panic!("unknown server");
        }
    }

    // The distribution is not perfect, but it should be close to 50/50
    // If it's not, then the load balancer is not working
    // This allows for a 10% error margin
    assert!(server_1 > NUM_REQUESTS / 2 - NUM_REQUESTS / 10);
    assert!(server_2 > NUM_REQUESTS / 2 - NUM_REQUESTS / 10);
    assert_eq!(server_1 + server_2, NUM_REQUESTS);
}

#[tokio::test]
async fn test_dns_resolve_v4() {
    let addr_1 = SocketAddr::from((
        [127, 0, 0, 1],
        portpicker::pick_unused_port().expect("failed to pick port"),
    ));
    tokio::spawn(
        Server::builder()
            .add_service(pb::test_server::TestServer::new(TestImpl {
                name: "server1".to_string(),
            }))
            .serve(addr_1),
    );

    let addr_2 = SocketAddr::from((
        [127, 0, 0, 1],
        portpicker::pick_unused_port().expect("failed to pick port"),
    ));
    tokio::spawn(
        Server::builder()
            .add_service(pb::test_server::TestServer::new(TestImpl {
                name: "server2".to_string(),
            }))
            .serve(addr_2),
    );

    let channel = make_channel_with_opts(ChannelOpts {
        addresses: vec![
            format!("localhost:{}", addr_1.port()),
            format!("localhost:{}", addr_2.port()),
        ],
        try_cname: false,
        enable_ipv6: false,
        enable_ipv4: true,
        interval: Duration::from_secs(0),
        tls: None,
    })
    .unwrap();
    let mut client = pb::test_client::TestClient::new(channel);

    let mut server_1 = 0;
    let mut server_2 = 0;

    const NUM_REQUESTS: usize = 1000;

    for _ in 0..NUM_REQUESTS {
        let response = client
            .test(tonic::Request::new(pb::TestRequest {
                message: "test".to_string(),
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.message, "test");

        if response.server == "server1" {
            server_1 += 1;
        } else if response.server == "server2" {
            server_2 += 1;
        } else {
            panic!("unknown server");
        }
    }

    // The distribution is not perfect, but it should be close to 50/50
    // If it's not, then the load balancer is not working
    // This allows for a 10% error margin
    assert!(server_1 > NUM_REQUESTS / 2 - NUM_REQUESTS / 10);
    assert!(server_2 > NUM_REQUESTS / 2 - NUM_REQUESTS / 10);
    assert_eq!(server_1 + server_2, NUM_REQUESTS);
}

#[tokio::test]
async fn test_dns_resolve_v6() {
    let addr_1 = SocketAddr::from((
        [0, 0, 0, 0, 0, 0, 0, 1],
        portpicker::pick_unused_port().expect("failed to pick port"),
    ));
    tokio::spawn(
        Server::builder()
            .add_service(pb::test_server::TestServer::new(TestImpl {
                name: "server1".to_string(),
            }))
            .serve(addr_1),
    );

    let addr_2 = SocketAddr::from((
        [0, 0, 0, 0, 0, 0, 0, 1],
        portpicker::pick_unused_port().expect("failed to pick port"),
    ));
    tokio::spawn(
        Server::builder()
            .add_service(pb::test_server::TestServer::new(TestImpl {
                name: "server2".to_string(),
            }))
            .serve(addr_2),
    );

    let channel = make_channel_with_opts(ChannelOpts {
        addresses: vec![
            format!("localhost:{}", addr_1.port()),
            format!("localhost:{}", addr_2.port()),
        ],
        try_cname: false,
        enable_ipv6: true,
        enable_ipv4: false,
        interval: Duration::from_secs(0),
        tls: None,
    })
    .unwrap();
    let mut client = pb::test_client::TestClient::new(channel);

    let mut server_1 = 0;
    let mut server_2 = 0;

    const NUM_REQUESTS: usize = 1000;

    for _ in 0..NUM_REQUESTS {
        let response = client
            .test(tonic::Request::new(pb::TestRequest {
                message: "test".to_string(),
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.message, "test");

        if response.server == "server1" {
            server_1 += 1;
        } else if response.server == "server2" {
            server_2 += 1;
        } else {
            panic!("unknown server");
        }
    }

    // The distribution is not perfect, but it should be close to 50/50
    // If it's not, then the load balancer is not working
    // This allows for a 10% error margin
    assert!(server_1 > NUM_REQUESTS / 2 - NUM_REQUESTS / 10);
    assert!(server_2 > NUM_REQUESTS / 2 - NUM_REQUESTS / 10);
    assert_eq!(server_1 + server_2, NUM_REQUESTS);
}

#[tokio::test]
async fn test_dns_resolve_cname() {
    struct Dns;

    #[async_trait]
    impl DnsResolver for Dns {
        async fn lookup(
            &self,
            hostname: &str,
            record_type: RecordType,
        ) -> Result<Lookup, ResolveError> {
            assert_eq!(hostname, "localhost");
            assert_eq!(record_type, RecordType::CNAME);

            Ok(Lookup::new_with_max_ttl(
                Query::new(),
                Arc::from([Record::from_rdata(
                    Name::default(),
                    0,
                    RData::CNAME(CNAME(Name::from_utf8("localhost").unwrap())),
                )]),
            ))
        }
    }

    let addr = SocketAddr::from((
        [127, 0, 0, 1],
        portpicker::pick_unused_port().expect("failed to pick port"),
    ));
    tokio::spawn(
        Server::builder()
            .add_service(pb::test_server::TestServer::new(TestImpl {
                name: "server1".to_string(),
            }))
            .serve(addr),
    );

    let channel = make_channel_with_resolver(
        Dns,
        ChannelOpts {
            addresses: vec![format!("localhost:{}", addr.port())],
            enable_ipv4: false,
            enable_ipv6: false,
            try_cname: true,
            interval: Duration::from_millis(0),
            tls: None,
        },
    )
    .unwrap();

    let mut client = pb::test_client::TestClient::new(channel);

    let response = client
        .test(tonic::Request::new(pb::TestRequest {
            message: "test".to_string(),
        }))
        .await
        .unwrap()
        .into_inner();

    assert_eq!(response.message, "test");
    assert_eq!(response.server, "server1");
}

#[tokio::test]
async fn test_headless_dns_resolve() {
    struct Dns {
        addresses: Vec<SocketAddr>,
    }

    #[async_trait]
    impl DnsResolver for Dns {
        async fn lookup(
            &self,
            hostname: &str,
            record_type: RecordType,
        ) -> Result<Lookup, ResolveError> {
            assert_eq!(hostname, "localhost");
            assert_eq!(record_type, RecordType::CNAME);

            let records = self
                .addresses
                .iter()
                .map(|addr| {
                    Record::from_rdata(
                        Name::default(),
                        0,
                        match addr.ip() {
                            IpAddr::V4(addr) => RData::A(A(addr)),
                            IpAddr::V6(addr) => RData::AAAA(AAAA(addr)),
                        },
                    )
                })
                .collect::<Vec<_>>();

            Ok(Lookup::new_with_max_ttl(Query::new(), Arc::from(records)))
        }
    }

    let port = portpicker::pick_unused_port().expect("failed to pick port");

    let addr_1 = SocketAddr::from(([127, 0, 0, 1], port));
    tokio::spawn(
        Server::builder()
            .add_service(pb::test_server::TestServer::new(TestImpl {
                name: "server1".to_string(),
            }))
            .serve(addr_1),
    );

    let addr_2 = SocketAddr::from(([127, 0, 0, 2], port));
    tokio::spawn(
        Server::builder()
            .add_service(pb::test_server::TestServer::new(TestImpl {
                name: "server2".to_string(),
            }))
            .serve(addr_2),
    );

    let resolver = Dns {
        addresses: vec![addr_1, addr_2],
    };

    let channel = make_channel_with_resolver(
        resolver,
        ChannelOpts {
            addresses: vec![format!("localhost:{}", port)],
            enable_ipv4: true,
            enable_ipv6: true,
            try_cname: true,
            interval: Duration::from_secs(0),
            tls: None,
        },
    )
    .unwrap();
    let mut client = pb::test_client::TestClient::new(channel);

    let mut server_1 = 0;
    let mut server_2 = 0;

    const NUM_REQUESTS: usize = 1000;

    for _ in 0..NUM_REQUESTS {
        let response = client
            .test(tonic::Request::new(pb::TestRequest {
                message: "test".to_string(),
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.message, "test");

        if response.server == "server1" {
            server_1 += 1;
        } else if response.server == "server2" {
            server_2 += 1;
        } else {
            panic!("unknown server");
        }
    }

    // The distribution is not perfect, but it should be close to 50/50
    // If it's not, then the load balancer is not working
    // This allows for a 10% error margin
    assert!(server_1 > NUM_REQUESTS / 2 - NUM_REQUESTS / 10);
    assert!(server_2 > NUM_REQUESTS / 2 - NUM_REQUESTS / 10);
    assert_eq!(server_1 + server_2, NUM_REQUESTS);
}

#[tokio::test]
async fn test_dns_resolve_change() {
    struct Dns {
        addresses: Arc<Mutex<Vec<SocketAddr>>>,
    }

    #[async_trait]
    impl DnsResolver for Dns {
        async fn lookup(
            &self,
            hostname: &str,
            record_type: RecordType,
        ) -> Result<Lookup, ResolveError> {
            assert_eq!(hostname, "localhost");
            assert_eq!(record_type, RecordType::CNAME);

            let records = self
                .addresses
                .lock()
                .await
                .iter()
                .map(|addr| {
                    Record::from_rdata(
                        Name::default(),
                        0,
                        match addr.ip() {
                            IpAddr::V4(addr) => RData::A(A(addr)),
                            IpAddr::V6(addr) => RData::AAAA(AAAA(addr)),
                        },
                    )
                })
                .collect::<Vec<_>>();

            Ok(Lookup::new_with_max_ttl(Query::new(), Arc::from(records)))
        }
    }

    let port = portpicker::pick_unused_port().expect("failed to pick port");

    let addr_1 = SocketAddr::from(([127, 0, 0, 1], port));
    tokio::spawn(
        Server::builder()
            .add_service(pb::test_server::TestServer::new(TestImpl {
                name: "server1".to_string(),
            }))
            .serve(addr_1),
    );

    let addr_2 = SocketAddr::from(([127, 0, 0, 2], port));
    tokio::spawn(
        Server::builder()
            .add_service(pb::test_server::TestServer::new(TestImpl {
                name: "server2".to_string(),
            }))
            .serve(addr_2),
    );

    let addresses = Arc::new(Mutex::new(vec![addr_1, addr_2]));

    let resolver = Dns {
        addresses: addresses.clone(),
    };

    let channel = make_channel_with_resolver(
        resolver,
        ChannelOpts {
            addresses: vec![format!("localhost:{}", port)],
            enable_ipv4: true,
            enable_ipv6: true,
            try_cname: true,
            interval: Duration::from_millis(100), // very fast poll interval
            tls: None,
        },
    )
    .unwrap();
    let mut client = pb::test_client::TestClient::new(channel);

    let mut server_1 = 0;
    let mut server_2 = 0;

    const NUM_REQUESTS: usize = 1000;

    for _ in 0..NUM_REQUESTS {
        let response = client
            .test(tonic::Request::new(pb::TestRequest {
                message: "test".to_string(),
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.message, "test");

        if response.server == "server1" {
            server_1 += 1;
        } else if response.server == "server2" {
            server_2 += 1;
        } else {
            panic!("unknown server");
        }
    }

    // The distribution is not perfect, but it should be close to 50/50
    // If it's not, then the load balancer is not working
    // This allows for a 10% error margin
    assert!(server_1 > NUM_REQUESTS / 2 - NUM_REQUESTS / 10);
    assert!(server_2 > NUM_REQUESTS / 2 - NUM_REQUESTS / 10);
    assert_eq!(server_1 + server_2, NUM_REQUESTS);

    // Now remove the second server
    addresses.lock().await.remove(1);

    // Wait for the server to be removed
    tokio::time::sleep(Duration::from_millis(150)).await;

    let mut server_1 = 0;

    for _ in 0..NUM_REQUESTS {
        let response = client
            .test(tonic::Request::new(pb::TestRequest {
                message: "test".to_string(),
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.message, "test");

        if response.server == "server1" {
            server_1 += 1;
        } else {
            panic!("unknown server");
        }
    }

    // The distribution is not perfect, but it should be close to 100/0
    // If it's not, then the load balancer is not working
    assert_eq!(server_1, NUM_REQUESTS);
}
