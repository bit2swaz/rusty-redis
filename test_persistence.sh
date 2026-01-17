#!/bin/bash

echo "=========================================="
echo "  Redis Persistence Test"
echo "=========================================="
echo ""

# Clean up any existing dump
rm -f dump.rdb

# Start server
echo "[1] Starting fresh server..."
cargo run > /tmp/redis_persist_test.log 2>&1 &
SERVER_PID=$!
sleep 2

# Check if server started clean
if grep -q "No dump file found" /tmp/redis_persist_test.log; then
    echo "    ✓ Server started with empty database"
else
    echo "    ✗ Expected empty database message"
fi

echo ""
echo "[2] Setting 5 keys..."
for i in {1..5}; do
    echo -e "*3\r\n\$3\r\nSET\r\n\$4\r\nkey$i\r\n\$6\r\nvalue$i\r\n" | nc localhost 6379 > /dev/null
done
echo "    ✓ Set key1-key5"

echo ""
echo "[3] Verifying keys exist..."
RESULT=$(echo -e '*2\r\n$3\r\nGET\r\n$4\r\nkey3\r\n' | nc localhost 6379)
if [[ "$RESULT" == *"value3"* ]]; then
    echo "    ✓ key3 = value3"
else
    echo "    ✗ Failed to get key3"
fi

echo ""
echo "[4] Triggering SAVE..."
SAVE_RESULT=$(echo -e '*1\r\n$4\r\nSAVE\r\n' | nc localhost 6379)
if [[ "$SAVE_RESULT" == "+OK" ]]; then
    echo "    ✓ Database saved"
    ls -lh dump.rdb | awk '{print "    Dump file size: " $5}'
else
    echo "    ✗ SAVE command failed"
fi

echo ""
echo "[5] Stopping server..."
kill $SERVER_PID
wait $SERVER_PID 2>/dev/null
sleep 1
echo "    ✓ Server stopped"

echo ""
echo "[6] Restarting server..."
cargo run > /tmp/redis_persist_reload.log 2>&1 &
SERVER_PID=$!
sleep 2

# Check if data was loaded
LOADED=$(grep -i "loaded.*keys from disk" /tmp/redis_persist_reload.log | tail -1)
if [ ! -z "$LOADED" ]; then
    echo "    ✓ $LOADED"
else
    echo "    ✗ No load message found"
fi

echo ""
echo "[7] Verifying persisted data..."
for i in 1 3 5; do
    RESULT=$(echo -e "*2\r\n\$3\r\nGET\r\n\$4\r\nkey$i\r\n" | nc localhost 6379)
    if [[ "$RESULT" == *"value$i"* ]]; then
        echo "    ✓ key$i = value$i (restored from disk)"
    else
        echo "    ✗ key$i not found"
    fi
done

echo ""
echo "[8] Cleaning up..."
kill $SERVER_PID
wait $SERVER_PID 2>/dev/null
echo "    ✓ Server stopped"

echo ""
echo "=========================================="
echo "  Persistence Test Complete!"
echo "=========================================="
