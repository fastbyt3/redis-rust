# Build your own redis

## Respond to ping

- Ping msg: `PING <msg>`
- returns `PONG` if no arg is passed
- used to test if connection is alive, if server can serve data and measure latency

```
redis> PING
"PONG"
redis> PING "hello world"
"hello world"
```

- Our response must be encoded using `RESP -> Redis Serialization Protocol`
    - This protocol is what Redis clients use to interact with redis

- Adv of RESP:
    - simple
    - fast to parse
    - human readable

- `\r\n` (CRLF) is the protocol's terminator -> separates its parts

```
*1\r\n$4\r\nping\r\n

*1 -> arr of len 1

$4 -> string of size 4 (ping)
```

- Response to PING => [`+PONG\r\n`](`+pong\r\n`.md)

### Simple strings

- Encoded as `+` character followed by string
- mustnt contain `\r` or `\n` and is terminated by `\r\n`

```
+OK\r\n

+PONG\r\n
```

### Simple errors

- Instead of `+` we use `-`

```
-Error msg\r\n
```

## Responding to ECHO

```
*2\r\n$4\r\nECHO\r\n$3\r\nhey\r\n

*2 -> arr of len 2 ([echo, hey])

$4 -> string of size 4 (echo)

$3 -> string of size 3 (hey)

RESP: $3\r\nhey\r\n
```

## GET and SET cmds

```
redis> SET mykey "Hello"
"OK"
redis> GET mykey
"Hello"
redis> SET anotherkey "will expire in a minute" EX 60
"OK"
```
 
- responding with Simple string: `+OK\r\n` -> the key was set

- responding with Bulk string: `$3\r\nfoo\r\n` -> value of some key
- responding with Null (also a simple string): `_\r\n` -> if GET of some key is not present

### Handling concurrent requests for GET and SET

- Create a common store: `HashMap<String, String>`
- initiate in `main()`
- provide a deep copy to each tokio instance

- Read-Write locks: to ensure concurrent access is properly sync (prevent races / deadlocks)

## Adding EXPIRY feat for SETing a key

- expiry provided in `ms` using `PX` argument to SET

```bash
# First, it'll set a key with an expiry (100 milliseconds in this example)
$ redis-cli set random_key random_value px 100

# Immediately after, it'll send a GET command to retrieve the value
# The response to this should be "random_value" (encoded as a RESP bulk string)
$ redis-cli get random_key

# Then, it'll wait for the key to expire and send another GET command
# The response to this should be `-1\r\n` (a "null bulk string")
$ sleep 0.2 && redis-cli get random_key
```

- How to keep track of EXPIRY of kv pairs in store....
    - separate thread -> expensive??
        - cleanup fn ran every 5sec
    - how to clear the value from store....
        - checking for expiry time at each GET -> If expired remove val and don't return
        - if we dont GET and expiry time passes value never gets dealloc

- passive expiry -> checking if a value is expired and purging it if its expired
- active expiry -> check with a bg process and auto delete if expiry period is crossed

[How redis expires keys](https://redis.io/commands/expire/#how-redis-expires-keys) -> Reference

### Implementing Active expiry

1. `tokio::spawn` -> initiate a new task
2. Set sample_size for reservoir sampling (other sampling methods can be used)
3. Get random subset of keys from collection & check for expiry
4. IF number of keys in subset found expired greater than 25% -> Reinitiate

## Implementing Persistance using RDB

- Redis uses `.rdb` files for persistance
- config values:
    - `dir`: dir where RDB files are stored
    - `dbfilename`: name of RDB file

- getting these values from Redis CLI 

```
redis-cli CONFIG GET dir
*2\r\n$3\r\ndir\r\n$16\r\n/tmp/redis-files\r\n

redis-cli CONFIG GET dbfilename
*2\r\n$3\r\ndir\r\n$16\r\n/tmp/redis-files/file.rdb\r\n
```

- Expected response: Array with 2 Bulk strings - Key and value

