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

### Redis file format

- Ref: 
	- [Redis RDB file format](https://rdb.fnordig.de/file_format.html)
	- [RDB dumper](https://github.com/sripathikrishnan/redis-rdb-tools/wiki/Redis-RDB-Dump-File-Format)
- In general, objects are prefixed with their lengths, so before reading the object you know exactly how much memory to allocate.

- high level structure

```
----------------------------#
52 45 44 49 53              # Magic String "REDIS"
30 30 30 33                 # RDB Version Number as ASCII string. "0003" = 3
----------------------------
FA                          # Auxiliary field
$string-encoded-key         # May contain arbitrary metadata
$string-encoded-value       # such as Redis version, creation time, used memory, ...
----------------------------
FE 00                       # Indicates database selector. db number = 00
FB                          # Indicates a resizedb field
$length-encoded-int         # Size of the corresponding hash table
$length-encoded-int         # Size of the corresponding expire hash table
----------------------------# Key-Value pair starts
FD $unsigned-int            # "expiry time in seconds", followed by 4 byte unsigned int
$value-type                 # 1 byte flag indicating the type of value
$string-encoded-key         # The key, encoded as a redis string
$encoded-value              # The value, encoding depends on $value-type
----------------------------
FC $unsigned long           # "expiry time in ms", followed by 8 byte unsigned long
$value-type                 # 1 byte flag indicating the type of value
$string-encoded-key         # The key, encoded as a redis string
$encoded-value              # The value, encoding depends on $value-type
----------------------------
$value-type                 # key-value pair without expiry
$string-encoded-key
$encoded-value
----------------------------
FE $length-encoding         # Previous db ends, next db starts.
----------------------------
...                         # Additional key-value pairs, databases, ...

FF                          ## End of RDB file indicator
8-byte-checksum             ## CRC64 checksum of the entire file.
```

### OpCodes

| Byte | Name			|
| ---- | --------------	|
| 0xFF | EOF			|
| 0xFE | SELECTDB		|
| 0xFD | EXPIRETIME		|
| 0xFC | EXPIRETIMEMS	|
| 0xFB | RESIZEDB		|
| 0xFA | AUX			|

### Key-value pairs

- each KV has 4 parts:
    - Key expiry timestamp (optional) -> `0xFD` or `0xFC`
    - one byte flag -> indicates value type
    - key -> encoded as Redis String
    - value -> encoded acc to value type

#### Value types

1. 0 - String encoding
2. 1 - text 
3. 2 - set
4. 3 - Sorted set
5. 4 - HashMap
6. 9 - zipmap
7. 10 - ziplist
8. 11 - intset
9. 12 - sorted set in ziplist
10. 13 - hashmap in ziplist

- when value is one of 1, 2, 3 or 4, the value is a sequence of strings -> construct list, set, sorted set, hashmap
- one of 9, 10, 11 or 12, the value is wrapped in a string. after reading string it must be parsed further

#### Length encoding

- store the length of the next object in the stream
- length encoding is a variable byte length

- how it works:
    - read one byte -> 2 MSB are read
    - if starting bits are `00` -> next 6 bits represent length
    - `01` then additional byte is read -> combined 14 bits represent length
    - `10` -> discard remaining 6 bits -> additional 4 bytes are read which represent the length in Big endian format
    - `11` -> next object is encoded in special format & remaining 6 bits indicate format type

- result of this encoding
    1. Numbers upto and including 63 can be stored in 1 byte
    2. Numbers upto and including 16383 can be stored in 2 bytes
    3. Numbers upto 2^32 -1 can be stored in 5 bytes

#### String encoding

- no special End-of-string token is used
- kinda like Byte array

- 3 types
    - Length prefixed strings -> length encoding first done followed by raw string
    - 8, 16, 32 bit integer -> value of 6 bits (format type) after length encoding:
        - 0 -> 8 bit integer
		- 1 -> 16 bit integer
        - 2 -> 32 bit integer
    - LZF compressed string
        - format type (last 6 bits) = 4 (000100)
        - compressed length (clen) read thru length encoding
        - uncompressed length read thru length encoding
        - next `clen` bytes are read
        - decompress read bytes using LZF algo

### Accessing keys in RDB file

```bash
$ redis-cli keys "*"
# returns all keys as array
*1\r\n$3\r\nfoo\r\n # example where k-v is foo:foo
```
