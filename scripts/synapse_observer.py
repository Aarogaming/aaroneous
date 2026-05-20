import mmap
import os
import time
import struct

def observe_synapse(name="SAB_STORE", size=1024*1024):
    """
    Python 'Orchestrator' view of the Synaptic shared memory.
    This reads raw bytes written by Rust 'Executors'.
    """
    path = os.path.join(os.environ["LOCALAPPDATA"], "Temp", f"{name}.synapse")
    
    if not os.path.exists(path):
        print(f"[Python] Waiting for Synapse file: {path}")
        while not os.path.exists(path):
            time.sleep(1)

    with open(path, "r+b") as f:
        # Map the file into Python memory
        mm = mmap.mmap(f.fileno(), size)
        print(f"[Python] Synapse Linked: {path}")
        
        last_data = None
        try:
            while True:
                mm.seek(0)
                # Let's assume the first 4 bytes are a 'magic' or status code
                # and the rest is our payload.
                status_code = mm.read(4)
                data = mm.read(64).split(b'\x00')[0] # Read a chunk and trim nulls
                
                if data != last_data and data:
                    print(f"[Python Orchestrator] Signal Received from Rust Executor: {data.decode('utf-8', errors='ignore')}")
                    last_data = data
                
                time.sleep(0.1)
        except KeyboardInterrupt:
            mm.close()

if __name__ == "__main__":
    observe_synapse()
