"""B2E-PERCI-010: OPENAI_API_KEY was sent over plain http to any (non-loopback) host.

A fake LM server listens on this machine's non-loopback address. Perci is pointed
at it with an API key in the environment. The key must never arrive.
"""
import socket
import threading

from _perci_b2e import build, fail, log, ok, run_perci

CANARY = "sk-b2e-canary-0123456789"


def local_non_loopback_ip() -> str | None:
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    try:
        s.connect(("10.255.255.255", 1))  # no packet is sent for UDP connect
        ip = s.getsockname()[0]
    except OSError:
        ip = None
    finally:
        s.close()
    if ip and not ip.startswith("127."):
        return ip
    try:
        for info in socket.getaddrinfo(socket.gethostname(), None, socket.AF_INET):
            if not info[4][0].startswith("127."):
                return info[4][0]
    except OSError:
        pass
    return None


ip = local_non_loopback_ip()
if ip is None:
    fail("no non-loopback IPv4 address available to host the fake LM")
server = socket.socket()
server.bind((ip, 0))
server.listen(8)
server.settimeout(0.5)
port = server.getsockname()[1]
received: list[bytes] = []
stop = threading.Event()


def serve():
    while not stop.is_set():
        try:
            conn, _ = server.accept()
        except OSError:
            continue
        conn.settimeout(2)
        data = b""
        try:
            while b"\r\n\r\n" not in data and len(data) < 65536:
                chunk = conn.recv(4096)
                if not chunk:
                    break
                data += chunk
            received.append(data)
            body = b'{"choices":[{"message":{"content":"fake reply"}}]}'
            conn.sendall(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: "
                         + str(len(body)).encode() + b"\r\n\r\n" + body)
        except OSError:
            pass
        finally:
            conn.close()


thread = threading.Thread(target=serve, daemon=True)
thread.start()
exe = build()
url = f"http://{ip}:{port}/v1/chat/completions"
log(f"fake LM at {url}")
env = {"OPENAI_API_KEY": CANARY, "PERCI_ENABLE_EXTERNAL_LM": "1", "PERCI_MODEL_URL": url,
       "PERCI_MODEL_NAME": "fake"}
for prompt in ("explain how a hash map works", "tell me about rust lifetimes"):
    run_perci(exe, ["ask", "--no-weights", prompt], env_extra=env, timeout=60)
stop.set()
thread.join(timeout=5)
leaked = [r for r in received if CANARY.encode() in r]
log(f"fake LM saw {len(received)} request(s)")
if leaked:
    fail("OPENAI_API_KEY was sent in cleartext to a non-loopback host")
ok("no API key left the machine over plain http")
