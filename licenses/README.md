# Scuffle Licensing Strategy Overview

## TL;DR
Our software uses MIT or Apache 2.0 licenses for all library code, ensuring open source flexibility and permissiveness. For binaries or applications, we use the Business Source License (BSL) to protect our proprietary interests while promoting innovation. Under BSL, you're free to develop and operate digital platforms for user-generated content sharing and live streaming, as long as you're not creating, offering, or managing infrastructure services like managed content delivery networks (CDNs) or managed live streaming services aimed at third-party integration.

## Licenses Used

### MIT License
- **Scope:** All library code.
- **Key Features:** 
  - Very permissive, allowing commercial use, modification, distribution, and private use.
  - Requires inclusion of the original MIT license and copyright notice.
- **Best For:** Maximizing open source contribution and usage flexibility.

### Apache License 2.0
- **Scope:** All library code.
- **Key Features:**
  - Similar permissiveness to MIT, with an explicit grant of patent rights from contributors to users.
  - Requires preservation of copyright notice, disclaimer, and a copy of the license when redistributing the code.
- **Best For:** Projects that require an explicit patent license.

### Business Source License (BSL)
- **Scope:** Binaries or applications.
- **Key Features:**
  - Allows use of the software but with certain limitations to protect the licensor's commercial interests.
  - Converts to a more permissive license (e.g., Apache 2.0) after a certain period.
- **Additional Use Grant:**
    - Grants permission to develop and operate digital platforms for user-generated video content sharing and live streaming, intended for community-centric environments.
    - Explicitly prohibits the use for developing, operating, or providing managed CDN services or managed live streaming services that integrate the software for third-party use.
    - Aims to encourage innovation in user engagement and content creation platforms, while excluding uses for third-party content delivery and streaming infrastructure services.

## Summary
This licensing strategy is designed to foster an open and innovative development environment for library code while securing our proprietary applications with BSL. The Additional Use Grant under BSL ensures that our applications can be used to power creative and interactive platforms without enabling the creation of competitive managed service infrastructure.
