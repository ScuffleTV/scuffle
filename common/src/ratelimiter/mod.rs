use std::time::Duration;

use const_format::formatcp;
use fred::prelude::*;

const RATELIMIT_LUA: &str = include_str!("ratelimit.lua");
const _: () = {
    const START_WITH: &str = "#!lua name=";

    const PANIC_MESSAGE: &str = formatcp!(
        "The lua script must start with {START_WITH}, but it does not.\n\n{RATELIMIT_LUA}"
    );

    let start_with_bytes = START_WITH.as_bytes();
    let ratelimit_bytes = RATELIMIT_LUA.as_bytes();

    if ratelimit_bytes.len() < start_with_bytes.len() {
        panic!("{}", PANIC_MESSAGE);
    }

    // This is a hacky way to do a loop at compile time
    // Unfortunately, const fn's cannot have for loops
    let mut i = 0;
    loop {
        if i >= start_with_bytes.len() {
            break;
        }

        if ratelimit_bytes[i] != start_with_bytes[i] {
            panic!("{}", PANIC_MESSAGE);
        }

        i += 1;
    }
};

#[derive(Debug, Clone)]
pub struct RateLimiterOptions {
    /// The cost of the request
    pub cost: u32,
    /// The allowed quota for the duration
    pub quota: u32,
    /// How many requests can exceed the quota before the user is banned
    pub exceeded_limit: u32,
    /// The amount of time before the quota is reset in seconds
    pub quota_reset_seconds: u32,
    /// The amount of time before exceeded is reset in seconds
    pub exceeded_reset_seconds: u32,
    /// The amount of time before the user is unbanned in seconds
    pub banned_reset_seconds: u32,
    /// The namespace for the keys, you should change this to something unique.
    pub namespace: String,
    /// The key for the limit, defaults to "limit"
    pub limit_key: String,
    /// The key for the exceeded limit, defaults to "exceeded"
    pub exceeded_key: String,
    /// The key for the banned limit, defaults to "banned"
    pub banned_key: String,
}

#[derive(Debug, Clone, Default)]
pub struct RateLimitResponse {
    /// The quota remaining
    pub remaining: i64,
    /// If the user is banned
    pub banned: bool,
    /// When the quota will reset or the user will be unbanned, if None then the user will never be unbanned.
    pub reset: Option<Duration>,
}

impl RateLimitResponse {
    pub fn can_request(&self) -> bool {
        self.remaining >= 0 && !self.banned
    }
}

impl Default for RateLimiterOptions {
    fn default() -> Self {
        Self {
            cost: 1,
            quota: 10,
            exceeded_limit: 5,
            quota_reset_seconds: 10,
            exceeded_reset_seconds: 60,
            banned_reset_seconds: 300,
            namespace: "{ratelimit}:".to_string(),
            limit_key: "limit".to_string(),
            exceeded_key: "exceeded".to_string(),
            banned_key: "banned".to_string(),
        }
    }
}

pub async fn load_rate_limiter_script<R: FunctionInterface + Send + Sync>(
    client: &R,
) -> RedisResult<()> {
    client.function_load_cluster(true, RATELIMIT_LUA).await
}

pub async fn ratelimit<R: FunctionInterface + Send + Sync>(
    client: &R,
    options: &RateLimiterOptions,
) -> RedisResult<RateLimitResponse> {
    if options.cost > options.quota {
        return Err(RedisError::new(
            RedisErrorKind::InvalidArgument,
            "cost cannot be greater than quota",
        ));
    }

    let keys = vec![
        format!("{}{}", options.namespace, options.limit_key),
        format!("{}{}", options.namespace, options.exceeded_key),
        format!("{}{}", options.namespace, options.banned_key),
    ];

    let args = vec![
        options.cost.to_string(),
        options.quota.to_string(),
        options.exceeded_limit.to_string(),
        options.quota_reset_seconds.to_string(),
        options.exceeded_reset_seconds.to_string(),
        options.banned_reset_seconds.to_string(),
    ];

    let raw_response: RedisValue = client.fcall("ratelimit", keys, args).await?;

    match raw_response {
        RedisValue::Array(arr) => {
            if arr.len() != 3 {
                return Err(RedisError::new(
                    RedisErrorKind::Protocol,
                    "ratelimit script returned an invalid response",
                ));
            }

            let mut reset = None;
            let mut remaining = None;
            let mut banned = None;

            for value in arr {
                if let RedisValue::Array(keyvalue) = value {
                    if keyvalue.len() != 2 {
                        return Err(RedisError::new(
                            RedisErrorKind::Protocol,
                            "ratelimit script returned an invalid response",
                        ));
                    }

                    let key = &keyvalue[0];
                    let value = &keyvalue[1];

                    if let RedisValue::String(key) = key {
                        match key.as_bytes() {
                            b"remaining" => {
                                if let RedisValue::Integer(value) = value {
                                    remaining = Some(*value);
                                }
                            }
                            b"banned" => {
                                if let RedisValue::Integer(value) = value {
                                    banned = Some(*value != 0);
                                }
                            }
                            b"reset" => {
                                if let RedisValue::Integer(value) = value {
                                    reset = Some(*value);
                                }
                            }
                            _ => {}
                        }
                    }

                    continue;
                }

                return Err(RedisError::new(
                    RedisErrorKind::Protocol,
                    "ratelimit script returned an invalid response",
                ));
            }

            let (remaining, banned, reset) = match (remaining, banned, reset) {
                (Some(remaining), Some(banned), Some(reset)) => (remaining, banned, reset),
                _ => {
                    return Err(RedisError::new(
                        RedisErrorKind::Protocol,
                        "ratelimit script returned an invalid response",
                    ))
                }
            };

            Ok(RateLimitResponse {
                remaining,
                banned,
                reset: if reset >= 0 {
                    Some(Duration::from_secs(reset as u64))
                } else {
                    None
                },
            })
        }
        _ => Err(RedisError::new(
            RedisErrorKind::Protocol,
            "ratelimit script returned an invalid response",
        )),
    }
}
