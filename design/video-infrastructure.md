# Video Infrastructure Design

![](./assets/video-infrastructure.webp)

When a streamer goes live they hit the first microservice called ingest.

## Ingest

Ingest is responsible for handling the incoming go live requests. 

It is agnostic to the protocol used to go live, for example we can support:
- [WebRTC](https://en.wikipedia.org/wiki/WebRTC)
- [SRT](https://en.wikipedia.org/wiki/Secure_Reliable_Transport)
- [RTMP](https://en.wikipedia.org/wiki/Real-Time_Messaging_Protocol)

Once we get a new stream request we need to figure out who went live? What channel is the corrisponding stream request for (authentication), and what features the requested stream has access to.

We make a API call to the auth server to validate the incoming stream and then we make some API calls to create a stream target.

Once everything passes and we are all ready and set, we ingest the stream and convert it to a common protocol (SRT for example)

At this point we can also pass the stream to other optional or future ideas (such as transcription or image detection).

We then push the newly ingested stream onto the next stage: Transcoding.

## Transcoding

In the transcoding stage we fetch the stream from the ingest stage and then start creating video variants from it.

We can transcode it down into `1080p`, `720p`, `480p` or lower depending on what the specific requirements and features the stream has.

Transcoding is important since we can significantly reduce the bandwidth cost of the video by compressing it using codecs such as [H264](https://en.wikipedia.org/wiki/Advanced_Video_Coding), [VP8](https://en.wikipedia.org/wiki/VP8), [VP9](https://en.wikipedia.org/wiki/VP9) or [AV1](https://en.wikipedia.org/wiki/AV1). 
We can also decrease the image quality for viewers with slower connections or smaller displays. If the viewer has a slower connection we still want to provide them a stream to watch, and to do this we need to be able to provide a lower quality stream for them. Or if they have a smaller display providing a higher resolution stream does not make any difference and can in some cases look worse.

### Video Codecs

There are a few video codes mentioned in the above paragraph. Generally H264 has been the golden standard for most if not all of live video for a very long time, due to the large amount of client support. However AV1 provides much better quality per bit compression. On average it results in about 50% smaller files. So a 8k bitrate in H264 would be a 4k bitrate stream in AV1. This allows us to ship higher quality video to viewers with slower internet speeds and also allows us to save a lot of money in CDN caching and bandwidth costs. However AV1 is much harder to transcode than H264. So transcoding becomes a higher cost.  

## Edge

We are going to have many servers in multiple regions around the world like Europe, North America, South America, South East Asia, Australia, Africa.

These servers will cache the video locally so that viewers from those regions can get the lowest latency to broadcast possible.

Caching the video locally also means that viewers can also watch at a higher quality since the bandwidth will be local and is likely to be faster & cheaper than international bandwidth.

Edge is a very complex system designed to do these functions efficiently, reliably and cost effectively. So for that reason there is a [seperate flowchart diagram for edge](./cdn-edge.md).

## Website

The website will have a video player which will be able to playback the stream in the requested quality.


## Useful links

### Transport Protocols
- [What is WebRTC?](https://bloggeek.me/what-is-webrtc/)
- [What is SRT?](https://www.matrox.com/en/video/media/guides-articles/srt-protocol)
- [What is RTMP?](https://www.dacast.com/blog/rtmp-real-time-messaging-protocol)

### Video Codecs
- [What is H264?](https://www.haivision.com/resources/streaming-video-definitions/h-264)
- [What is VP8?](https://trueconf.com/blog/wiki/vp8-video-codec)
- [What is VP9?](https://www.wowza.com/blog/vp9-codec-googles-open-source-technology-explained)
- [What is AV1?](https://www.androidauthority.com/av1-codec-1113318)
- [H264 vs AV1](https://www.winxdvd.com/convert-hevc-video/av1-vs-hevc.htm)
- [VP9 vs AV1](https://www.winxdvd.com/video-transcoder/av1-vs-vp9.htm)
- [AV1 ASIC Encoding](https://netint.com/)
