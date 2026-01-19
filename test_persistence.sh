#!/bin/bash

echo "starting server..."
rm -f dump.rdb
./target/release/rusty-redis > /tmp/persist_test.log 2>&1 &
SERVER_PID=$!
sleep 2

echo "setting keys..."
echo -e '*3\r\n$3\r\nSET\r\n$7\r\npersist\r\n$5\r\nvalue\r\n' | nc localhost 6379
echo -e '*3\r\n$3\r\nSET\r\n$4\r\ntest\r\n$4\r\ndata\r\n' | nc localhost 6379

echo ""
echo "keys set. checking database has entries..."
echo -e '*2\r\n$3\r\nGET\r\n$7\r\npersist\r\n' | nc localhost 6379

echo ""
echo "stopping server with ctrl+c (sending sigint)..."
kill -INT $SERVER_PID
sleep 2

echo ""
echo "checking if dump.rdb was created..."
if [ -f dump.rdb ]; then
    echo "✓ dump.rdb exists ($(stat -c%s dump.rdb) bytes)"
    echo ""
    echo "restarting server..."
    ./target/release/rusty-redis > /tmp/persist_test2.log 2>&1 &
    SERVER_PID=$!
    sleep 2
    
    echo "checking if keys were loaded..."
    grep "loaded.*keys" /tmp/persist_test2.log
    
    echo ""
    echo "retrieving persisted key..."
    echo -e '*2\r\n$3\r\nGET\r\n$7\r\npersist\r\n' | nc localhost 6379
    
    kill $SERVER_PID
    wait $SERVER_PID 2>/dev/null
    echo ""
    echo "✓ persistence test complete!"
else
    echo "✗ dump.rdb was not created"
fi
