#!/usr/bin/env python3
import socket
import time
from multiprocessing import Process, Queue

def resp_set(key, value):
    return f"*3\r\n$3\r\nSET\r\n${len(key)}\r\n{key}\r\n${len(value)}\r\n{value}\r\n".encode()

def resp_get(key):
    return f"*2\r\n$3\r\nGET\r\n${len(key)}\r\n{key}\r\n".encode()

def worker(worker_id, ops_per_worker, queue):
    start = time.time()
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(("localhost", 6379))
    
    for i in range(ops_per_worker):
        key = f"key_{worker_id}_{i}"
        value = f"value_{i}"
        
        sock.sendall(resp_set(key, value))
        sock.recv(1024)
        
        sock.sendall(resp_get(key))
        sock.recv(1024)
    
    sock.close()
    elapsed = time.time() - start
    queue.put(elapsed)

def main():
    total_ops = 100000
    num_workers = 10
    ops_per_worker = total_ops // num_workers
    
    print(f"running benchmark: {total_ops} operations across {num_workers} workers")
    
    queue = Queue()
    processes = []
    
    start_time = time.time()
    
    for i in range(num_workers):
        p = Process(target=worker, args=(i, ops_per_worker, queue))
        p.start()
        processes.append(p)
    
    for p in processes:
        p.join()
    
    total_time = time.time() - start_time
    
    slowest = 0
    while not queue.empty():
        worker_time = queue.get()
        if worker_time > slowest:
            slowest = worker_time
    
    throughput = total_ops / total_time
    
    print(f"\ntotal time: {total_time:.2f} seconds")
    print(f"slowest worker: {slowest:.2f} seconds")
    print(f"throughput: {throughput:,.0f} ops/sec")
    
    if throughput > 50000:
        print("\n✓ SUCCESS: exceeded 50,000 ops/sec target!")
    else:
        print(f"\n✗ FAILED: did not reach 50,000 ops/sec target")

if __name__ == "__main__":
    main()
