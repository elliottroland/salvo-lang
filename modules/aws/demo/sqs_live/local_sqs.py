#!/usr/bin/env python3
"""A tiny local stand-in for Amazon SQS, for running the demo without AWS.

Speaks the awsJson1.0 protocol the SDKs use for SQS (POST / with an
`X-Amz-Target: AmazonSQS.<Operation>` header and a JSON body) for the six
operations `aws.sqs` generates, keeping queues in memory. Signatures are not
checked. Python 3's standard library only.

    python3 local_sqs.py            # listens on http://localhost:4566
"""

import hashlib
import json
import sys
import uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 4566
QUEUES = {}  # url -> list of messages
# SQS is `awsQueryCompatible`: an error also carries its legacy query code in
# the `x-amzn-query-error` header, which aws-sdk-rust matches on.
QUERY_CODES = {
    "QueueDoesNotExist": "AWS.SimpleQueueService.NonExistentQueue",
    "UnsupportedOperation": "AWS.SimpleQueueService.UnsupportedOperation",
}


def md5(text):
    return hashlib.md5(text.encode("utf-8")).hexdigest()


class Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):  # quiet
        pass

    def answer(self, status, body, query_code=None):
        data = json.dumps(body).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/x-amz-json-1.0")
        if query_code:
            self.send_header("x-amzn-query-error", query_code + ";Sender")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def error(self, code, message):
        self.answer(400, {"__type": "com.amazonaws.sqs#" + code, "message": message},
                    QUERY_CODES.get(code))

    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        req = json.loads(self.rfile.read(length) or b"{}")
        op = self.headers.get("X-Amz-Target", "").split(".")[-1]
        host = self.headers.get("Host", "localhost:%d" % PORT)
        if op == "CreateQueue":
            url = "http://%s/000000000000/%s" % (host, req["QueueName"])
            QUEUES.setdefault(url, [])
            return self.answer(200, {"QueueUrl": url})
        if op == "GetQueueUrl":
            for url in QUEUES:
                if url.endswith("/" + req["QueueName"]):
                    return self.answer(200, {"QueueUrl": url})
            return self.error("QueueDoesNotExist", "no queue named " + req["QueueName"])
        url = req.get("QueueUrl", "")
        if url not in QUEUES:
            return self.error("QueueDoesNotExist", "no queue at " + url)
        if op == "SendMessage":
            mid = str(uuid.uuid4())
            QUEUES[url].append({"MessageId": mid, "Body": req["MessageBody"]})
            return self.answer(200, {"MessageId": mid, "MD5OfMessageBody": md5(req["MessageBody"])})
        if op == "ReceiveMessage":
            n = req.get("MaxNumberOfMessages", 1)
            taken = QUEUES[url][:n]
            messages = [
                {
                    "MessageId": m["MessageId"],
                    "ReceiptHandle": m["MessageId"],
                    "Body": m["Body"],
                    "MD5OfBody": md5(m["Body"]),
                }
                for m in taken
            ]
            return self.answer(200, {"Messages": messages} if messages else {})
        if op == "DeleteMessage":
            QUEUES[url] = [m for m in QUEUES[url] if m["MessageId"] != req["ReceiptHandle"]]
            return self.answer(200, {})
        if op == "DeleteQueue":
            del QUEUES[url]
            return self.answer(200, {})
        return self.error("UnsupportedOperation", op + " is not in this stand-in")


if __name__ == "__main__":
    print("local SQS on http://localhost:%d" % PORT, flush=True)
    ThreadingHTTPServer(("localhost", PORT), Handler).serve_forever()
