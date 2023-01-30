# CDN Edge Design

On this diagram the arrow points in the direction of who instantiated the request. The thickness represents how much bandwidth there is.

![](./assets/cdn-edge.webp)

## Internet

The internet is where all incoming requests will come from. When a viewer requests a video segment from us, this is where the request lifecycle starts.

## Layer 1

Layer 1 is the first port of entry; all users will talk to a Layer 1 cache server.

Layer 1 cache servers will have very high outgoing bandwidth. This is because most, if not all, of the files we serve will be cache hits since everyone gets identical video files. However, we will have to serve a large number of people; therefore, we will need a lot of bandwidth at the edge of our network.

Layer 1 servers will be everywhere, in every region, and in multiple cities in each region. This is because we want to serve the people as close as possible.

Layer 1 servers will connect to 2 unique Layer 2 servers. This is to provide redundancy if a Layer 2 server goes down.

## Layer 2

Layer 2 servers will act as a 2nd cache Layer and relay instructions to the Layer 1 servers received from the control servers.

Layer 2 servers will connect to one Layer 2 server in each region (i.e., if we have 3x Layer 2 server in NA and 3x Layer 2 servers, each Layer 2 server in NA will connect to 1 Layer 2 server in EU and vice-versa)

Layer 2 will also act as a router. If we receive a request from a Layer 1 server to go to an origin and the Layer 2 server does not have the data. If the Layer 2 server can fetch the data from an origin, it will be routed to the origin. If it cannot, then the request will be routed to the Layer 2 server, which has an origin attached, and then the origin will receive the request.

In terms of network Hops.
- Layer 1 (cache miss)
- Layer 2 (cache miss)
- Layer 2 (cache miss & no direct connection)
- Origin (source of truth)

If Layer 2 in Hop 2 directly connects to the origin, we will route directly to Hop 4 (skipping over Hop 3).

## Control

The control servers will act as the state of the system. They will contain all the information about what routes exist in the network. They will also serve as an authentication and state management server for the Layer 2 servers, which will act as a manager for Layer 1 servers.

Control servers are entirely isolated from the request lifecycle and therefore do not need a large bandwidth pipe.

## Origin 

Origin servers will be the source of truth for all the data. If all Layers above cache miss, the origin will provide a fresh copy of the data for the Layers above to use.

## FAQ

### __What is this?__

_This is a straightforward CDN setup similar to that of Cloudflare or Fastly, or any other large CDN as a service provider._

### __Why build your own if very cheap and cost-effective solutions exist?__

_Well, they are not very cheap when looking at them at scale. They generally work for most cases, but video streaming is very expensive due to the large files, which makes the cheap solutions very expensive._

### __What makes this solution special?__

_This edge cache is tailored to the specific requirements of a video edge cache and has unique features that improve performance that other vendors do not offer._


## Unique Features

- Event stream updates from Origin. (uncache a file when it changes and push the file everywhere)
- Push to the edge (pre-push a file to the edge before it's requested)
- Request coalescing (merge multiple requests into a single request)
- Websocket/Stream coalescing (merge multiple Websockets/Streams into a single stream connection)
