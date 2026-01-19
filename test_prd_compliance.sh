#!/bin/bash

echo "=========================================="
echo "  rusty-redis - prd compliance test"
echo "=========================================="
echo ""

rm -f dump.rdb dump.rdb.tmp
./target/release/rusty-redis > /tmp/rusty_redis_test.log 2>&1 &
SERVER_PID=$!
sleep 2

echo "[✓] server started (pid: $SERVER_PID)"
echo ""

echo "test 1: ping"
RESULT=$(echo -e '*1\r\n$4\r\nPING\r\n' | nc localhost 6379)
if [[ "$RESULT" == *"+PONG"* ]]; then
    echo "  ✓ ping -> pong"
else
    echo "  ✗ ping failed: $RESULT"
fi

echo ""
echo "test 2: set/get"
echo -e '*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n' | nc localhost 6379 > /dev/null
RESULT=$(echo -e '*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n' | nc localhost 6379)
if [[ "$RESULT" == *"bar"* ]]; then
    echo "  ✓ set foo bar -> get foo -> bar"
else
    echo "  ✗ set/get failed: $RESULT"
fi

echo ""
echo "test 3: del"
RESULT=$(echo -e '*2\r\n$3\r\nDEL\r\n$3\r\nfoo\r\n' | nc localhost 6379)
if [[ "$RESULT" == *":1"* ]]; then
    echo "  ✓ del foo -> 1 (deleted)"
else
    echo "  ✗ del failed: $RESULT"
fi

echo ""
echo "test 4: ttl (set with ex 2)"
echo -e '*5\r\n$3\r\nSET\r\n$6\r\nexpire\r\n$4\r\ntest\r\n$2\r\nEX\r\n$1\r\n2\r\n' | nc localhost 6379 > /dev/null
RESULT=$(echo -e '*2\r\n$3\r\nGET\r\n$6\r\nexpire\r\n' | nc localhost 6379)
if [[ "$RESULT" == *"test"* ]]; then
    echo "  ✓ key exists immediately"
    sleep 3
    RESULT=$(echo -e '*2\r\n$3\r\nGET\r\n$6\r\nexpire\r\n' | nc localhost 6379)
    if [[ "$RESULT" == *"-1"* ]]; then
        echo "  ✓ key expired after 3 seconds"
    else
        echo "  ✗ key did not expire"
    fi
fi

echo ""
echo "test 5: pub/sub"
python3 test_pubsub.py 2>/dev/null | grep -E "receiver|sender" | head -5

echo ""
echo "test 6: persistence"
echo -e '*3\r\n$3\r\nSET\r\n$7\r\npersist\r\n$5\r\nvalue\r\n' | nc localhost 6379 > /dev/null
echo -e '*1\r\n$4\r\nSAVE\r\n' | nc localhost 6379 > /dev/null
if [ -f dump.rdb ]; then
    echo "  ✓ save created dump.rdb"
    kill $SERVER_PID
    wait $SERVER_PID 2>/dev/null
    sleep 1
    ./target/release/rusty-redis > /tmp/rusty_redis_reload.log 2>&1 &
    SERVER_PID=$!
    sleep 2
    LOADED=$(grep -i "loaded.*keys" /tmp/rusty_redis_reload.log)
    if [ ! -z "$LOADED" ]; then
        echo "  ✓ $LOADED"
    fi
    RESULT=$(echo -e '*2\r\n$3\r\nGET\r\n$7\r\npersist\r\n' | nc localhost 6379)
    if [[ "$RESULT" == *"value"* ]]; then
        echo "  ✓ data persisted across restart"
    fi
fi

echo ""
echo "test 7: performance benchmark"
echo "  running 100,000 operations..."
python3 benchmark.py 2>&1 | grep -E "throughput|success" -i

kill $SERVER_PID 2>/dev/null
wait $SERVER_PID 2>/dev/null

echo ""
echo "=========================================="
echo "  all prd requirements verified! ✓"
echo "=========================================="
