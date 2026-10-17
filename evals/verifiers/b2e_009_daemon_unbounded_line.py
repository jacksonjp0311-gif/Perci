"""B2E-PERCI-009: the chat daemon buffered an unbounded request line.

Send up to 32 MiB without a newline. The daemon must stop reading (close the connection)
long before it has buffered the whole payload, and keep serving new clients.
The old daemon reads everything until a newline; the fixed one answers
"payload too large" and closes after MAX_LINE_BYTES+1 bytes.
"""
import json
import select
import socket
import subprocess
import time

from _perci_b2e import build, clean_env, fail, ok

exe = build()
probe_sock = socket.socket()
probe_sock.bind(("127.0.0.1", 0))
port = probe_sock.getsockname()[1]
probe_sock.close()
env = clean_env({"PERCI_DAEMON_PORT": str(port), "PERCI_NO_WEIGHTS": "1"})
daemon = subprocess.Popen([str(exe), "daemon", "--no-weights"], env=env,
                          stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
try:
    deadline = time.time() + 60
    while True:
        try:
            socket.create_connection(("127.0.0.1", port), timeout=1).close()
            break
        except OSError:
            if daemon.poll() is not None or time.time() > deadline:
                fail("daemon did not start: " + daemon.stderr.read().decode("utf-8", "replace")[-500:])
            time.sleep(0.2)

    # 32 MiB without a newline. A bounded reader answers "payload too large" and
    # hangs up after ~256 KiB, i.e. while we are still sending. An unbounded
    # reader silently buffers all of it and only answers once we stop sending.
    total = 32 * 1024 * 1024
    chunk = b"a" * (64 * 1024)
    sent = 0
    early = False
    with socket.create_connection(("127.0.0.1", port), timeout=20) as conn:
        try:
            while sent < total:
                readable, _, _ = select.select([conn], [], [], 0)
                if readable:
                    early = True
                    break
                conn.sendall(chunk)
                sent += len(chunk)
        except OSError:
            early = True  # reset / broken pipe: the daemon hung up mid-stream
    print(f"[b2e] sent {sent} of {total} bytes before the daemon reacted (early={early})")

    # The daemon must still serve a normal client afterwards.
    with socket.create_connection(("127.0.0.1", port), timeout=20) as conn:
        conn.sendall(json.dumps({"op": "ping"}).encode() + b"\n")
        pong = conn.makefile("rb").readline()
    if b"ok" not in pong and b"pong" not in pong:
        fail(f"daemon stopped serving after the oversized line: {pong[:200]!r}")
    if not early or sent >= total // 2:
        fail("daemon kept buffering the oversized line instead of closing early")
    ok("daemon refuses oversized lines without buffering them")
finally:
    daemon.kill()
    daemon.wait(timeout=10)
