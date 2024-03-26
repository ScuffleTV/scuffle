#!lua name=ratelimiter

local function ratelimit_impl(limit_key, exceeded_key, banned_key, cost, quota, exceed_limit, quota_reset_seconds, exceeded_reset_seconds, banned_reset_seconds)
    local response = {}
    local ban_ttl = redis.call("TTL", banned_key)

    if ban_ttl ~= -2 then
        table.insert(response, {"quota", 0})
        table.insert(response, {"banned", 1})
        table.insert(response, {"reset", ban_ttl})
        return response
    end

    local current = redis.call("INCRBY", limit_key, cost)
    if current > quota then
        local exceeded = redis.call("INCR", exceeded_key)
        if exceeded > exceed_limit then
            redis.call("SET", banned_key, 1)
            redis.call("EXPIRE", banned_key, banned_reset_seconds)

            table.insert(response, {"remaining", -1})
            table.insert(response, {"banned", 1})
            table.insert(response, {"reset", banned_reset_seconds})
            return response
        elseif exceeded == 1 then
            redis.call("EXPIRE", exceeded_key, exceeded_reset_seconds)
        end

        local reset = redis.call("TTL", limit_key)

        table.insert(response, {"remaining", -1})
        table.insert(response, {"banned", 0})
        table.insert(response, {"reset", reset})
        return response
    end

    local reset = quota_reset_seconds
    if current == cost then
        redis.call("EXPIRE", limit_key, quota_reset_seconds)
    else
        reset = redis.call("TTL", limit_key)
    end

    table.insert(response, {"remaining", quota - current})
    table.insert(response, {"banned", 0})
    table.insert(response, {"reset", reset})
    return response
end

local function ratelimiter(keys, args)
    if #keys ~= 3 then
        return redis.error_reply("invalid number of keys, expected 3, got " .. #keys)
    end

    if #args ~= 6 then
        return redis.error_reply("invalid number of arguments, expected 5, got " .. #args)
    end

    local limit_key = keys[1]
    local exceeded_key = keys[2]
    local banned_key = keys[3]

    local cost = tonumber(args[1])
    if cost == nil then
        return redis.error_reply("invalid cost, expected number, got " .. type(args[1]))
    end

    local quota = tonumber(args[2])
    if quota == nil then
        return redis.error_reply("invalid quota, expected number, got " .. type(args[2]))
    end

    if cost > quota then
        return redis.error_reply("cost cannot be greater than quota")
    end

    local exceeded_limit = tonumber(args[3])
    if exceeded_limit == nil then
        return redis.error_reply("invalid exceeded_limit, expected number, got " .. type(args[3]))
    end

    local quota_reset_seconds = tonumber(args[4])
    if quota_reset_seconds == nil then
        return redis.error_reply("invalid quota_reset_seconds, expected number, got " .. type(args[4]))
    end

    local exceeded_reset_seconds = tonumber(args[5])
    if exceeded_reset_seconds == nil then
        return redis.error_reply("invalid exceeded_reset_seconds, expected number, got " .. type(args[5]))
    end

    local banned_reset_seconds = tonumber(args[6])
    if banned_reset_seconds == nil then
        return redis.error_reply("invalid banned_reset_seconds, expected number, got " .. type(args[6]))
    end

    return ratelimit_impl(limit_key, exceeded_key, banned_key, cost, quota, exceeded_limit, quota_reset_seconds, exceeded_reset_seconds, banned_reset_seconds)
end

redis.register_function{
    function_name="ratelimit",
    callback=ratelimiter
}
