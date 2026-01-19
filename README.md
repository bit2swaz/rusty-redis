# rusty-redis

a high-performance, concurrent in-memory key-value store implementing the redis serialization protocol (resp v2) in rust.

## overview

rusty-redis is a redis-compatible server built with async rust. unlike single-threaded redis, it uses tokio's async runtime and dashmap's lock-free concurrent hashmap to handle thousands of connections across multiple cpu cores without global locks.

_for a deep dive, feel free to read [this blog](https://bit2-blog.vercel.app/posts/rusty-redis-deep-dive) written by me_

## features

- **resp v2 protocol**: compatible with redis-cli and standard client libraries
- **concurrent access**: lock-free operations using dashmap sharding
- **key expiration**: probabilistic ttl eviction with background janitor task
- **pub/sub**: multi-producer, multi-consumer message channels
- **persistence**: atomic snapshot-based disk persistence (rdb-style)
- **auto-snapshot**: configurable interval-based automatic saves
- **async i/o**: fully non-blocking with tokio runtime
- **zero unsafe code**: memory-safe implementation

## performance

```
throughput: 241,983 ops/sec (100k operations benchmark)
```

## commands

| command | syntax | description |
|---------|--------|-------------|
| `PING` | `PING [msg]` | health check, returns pong or message |
| `SET` | `SET key val [EX seconds]` | store value with optional ttl |
| `GET` | `GET key` | retrieve value, returns nil if expired/missing |
| `DEL` | `DEL key` | delete key, returns 1 if deleted, 0 if not found |
| `PUBLISH` | `PUBLISH channel msg` | broadcast message to channel subscribers |
| `SUBSCRIBE` | `SUBSCRIBE channel` | enter pub/sub mode for channel |
| `SAVE` | `SAVE` | manually trigger snapshot |

## installation

```bash
git clone https://github.com/bit2swaz/rusty-redis.git
cd rusty-redis
cargo build --release
```

## usage

start the server:

```bash
./target/release/rusty-redis
```

connect with redis-cli:

```bash
redis-cli -p 6379
> PING
PONG
> SET hello world
OK
> GET hello
"world"
> SET temp value EX 5
OK
> GET temp
"value"
# wait 6 seconds
> GET temp
(nil)
```

## architecture

### core components

- **tcp listener**: accepts connections and spawns async tasks per connection
- **frame decoder**: parses raw bytes into resp frames (array, bulk, simple, integer, error, null)
- **storage engine**: `Arc<DashMap<String, Bytes>>` for concurrent access without global locks
- **expiry manager**: background task sampling 20 random keys every 100ms for eviction
- **persistence manager**: auto-snapshot every 60s if changes occurred, atomic writes via temp file

### concurrency model

```rust
pub struct Db {
    entries: Arc<DashMap<String, Bytes>>,
    expirations: Arc<DashMap<String, Instant>>,
    pub_sub: Arc<DashMap<String, broadcast::Sender<Bytes>>>,
    changed: Arc<AtomicBool>,
}
```

dashmap shards the hashmap into buckets (default: one per cpu core). write operations only lock a single bucket, allowing other threads to access different keys simultaneously.

### expiration algorithm

1. background task runs every 100ms
2. samples 20 random keys from expiration map
3. deletes expired keys from both entries and expirations
4. if >25% sampled keys expired, repeats immediately (memory pressure detection)

### persistence

snapshot format uses bincode binary serialization. write process:

1. iterate all entries in dashmap
2. serialize to `dump.rdb.tmp`
3. atomically rename to `dump.rdb` (prevents corruption on crash)
4. auto-snapshot runs every 60s if any changes occurred

## configuration

edit `src/main.rs` constants:

```rust
const EVICTION_INTERVAL: Duration = Duration::from_millis(100);
const SNAPSHOT_INTERVAL: Duration = Duration::from_secs(60);
const BIND_ADDR: &str = "127.0.0.1:6379";
```

## testing

run tests:

```bash
cargo test
```

manual verification:

```bash
# ping test
echo -e '*1\r\n$4\r\nPING\r\n' | nc localhost 6379

# set/get test
echo -e '*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n' | nc localhost 6379
echo -e '*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n' | nc localhost 6379

# run full compliance test
./test_prd_compliance.sh
```

benchmark:

```bash
python3 benchmark.py
```

## dependencies

```toml
tokio = { version = "1", features = ["full"] }
bytes = "1"
dashmap = "5"
serde = { version = "1", features = ["derive"] }
bincode = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
```

## project structure

```
src/
├── main.rs          # tcp listener, connection handler, command dispatch
├── frame.rs         # resp protocol frame types and serialization
├── connection.rs    # buffered tcp stream with frame read/write
├── db.rs            # storage engine with concurrent access
├── cmd.rs           # command parsing from frames
└── persistence.rs   # snapshot save/load with atomic writes
```

## limitations

- no cluster mode
- no lua scripting
- limited command set (core operations only)
- no aof (append-only file) persistence
- pub/sub messages not persisted

## contributing

this is a learning project demonstrating async rust patterns, shared-state concurrency, and non-blocking i/o.

## license

see [LICENSE](LICENSE) file.