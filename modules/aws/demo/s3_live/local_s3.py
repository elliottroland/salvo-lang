#!/usr/bin/env python3
"""A tiny local stand-in for Amazon S3, for running the demo without AWS.

Speaks the restXml protocol the SDKs use for S3, path-style
(`/<bucket>/<key>`, which `HostS3` asks for whenever an `endpoint` is set), for
the two operations `aws.s3` generates: `PutObject` and `GetObject`. Objects live
in memory, and a bucket exists as soon as something is put in it. Signatures
are not checked; checksums the SDKs send are accepted and not verified. Python
3's standard library only.

    python3 local_s3.py            # listens on http://localhost:4566
"""

import email.utils
import hashlib
import sys
import time
import uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import unquote, urlsplit

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 4566
OBJECTS = {}  # (bucket, key) -> {"data", "type", "etag", "modified"}


def aws_chunked(raw):
    """Decodes an `aws-chunked` body: `<hex size>[;ext]\\r\\n<data>\\r\\n` chunks,
    a zero-size chunk, then trailers (the checksum) and a blank line. Both SDKs
    send one when they stream a body with a trailing checksum."""
    out = bytearray()
    at = 0
    while True:
        end = raw.index(b"\r\n", at)
        size = int(raw[at:end].split(b";")[0], 16)
        at = end + 2
        if size == 0:
            return bytes(out)
        out += raw[at:at + size]
        at += size + 2


class Handler(BaseHTTPRequestHandler):
    # HTTP/1.1, so the SDKs' pooled connections stay open and `Expect:
    # 100-continue` is answered by the base class.
    protocol_version = "HTTP/1.1"

    def log_message(self, fmt, *args):  # quiet
        pass

    def target(self):
        path = unquote(urlsplit(self.path).path)
        bucket, _, key = path.lstrip("/").partition("/")
        return bucket, key

    def body(self):
        if self.headers.get("Transfer-Encoding", "").lower() == "chunked":
            raw = bytearray()
            while True:
                size = int(self.rfile.readline().split(b";")[0], 16)
                if size == 0:
                    while self.rfile.readline() not in (b"\r\n", b"\n", b""):
                        pass
                    break
                raw += self.rfile.read(size)
                self.rfile.readline()
            raw = bytes(raw)
        else:
            raw = self.rfile.read(int(self.headers.get("Content-Length", "0")))
        encoding = self.headers.get("Content-Encoding", "")
        signed = self.headers.get("x-amz-content-sha256", "")
        if "aws-chunked" in encoding or signed.startswith("STREAMING-"):
            return aws_chunked(raw)
        return raw

    def reply(self, status, headers=(), data=b""):
        self.send_response(status)
        self.send_header("x-amz-request-id", uuid.uuid4().hex[:16].upper())
        for name, value in headers:
            self.send_header(name, value)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(data)

    def error(self, status, code, message, key=""):
        xml = (
            '<?xml version="1.0" encoding="UTF-8"?>\n'
            "<Error><Code>%s</Code><Message>%s</Message><Key>%s</Key><RequestId>0</RequestId></Error>"
            % (code, message, key)
        ).encode("utf-8")
        self.reply(status, [("Content-Type", "application/xml")], xml)

    def do_PUT(self):
        bucket, key = self.target()
        data = self.body()
        if not key:
            return self.error(400, "NotImplemented", "only objects can be put here")
        etag = '"%s"' % hashlib.md5(data).hexdigest()
        OBJECTS[(bucket, key)] = {
            "data": data,
            "type": self.headers.get("Content-Type", "binary/octet-stream"),
            "etag": etag,
            "modified": email.utils.formatdate(time.time(), usegmt=True),
        }
        self.reply(200, [("ETag", etag)])

    def do_GET(self):
        bucket, key = self.target()
        found = OBJECTS.get((bucket, key))
        if found is None:
            return self.error(404, "NoSuchKey", "The specified key does not exist.", key)
        self.reply(200, [
            ("Content-Type", found["type"]),
            ("ETag", found["etag"]),
            ("Last-Modified", found["modified"]),
            ("Accept-Ranges", "bytes"),
        ], found["data"])

    do_HEAD = do_GET


if __name__ == "__main__":
    print("local S3 on http://localhost:%d" % PORT, flush=True)
    ThreadingHTTPServer(("localhost", PORT), Handler).serve_forever()
