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
