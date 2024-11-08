a simple clipboard server. the client was implemented in python

``` python
#!/usr/bin/env python3

import socket
import argparse
from json import JSONEncoder, JSONDecoder
from sys import stderr

def send(ss: str):
    s = socket.socket()
    s.connect(('127.0.0.1',33304))

    request = {
        "type": "copy",
        "contents": ss
    }

    json_string = JSONEncoder().encode(request)

    s.send(json_string.encode())
    s.close()

def request():
    s = socket.socket()
    s.connect(('127.0.0.1', 33304))

    request = {
        "type": "paste",
        "contents": "",
    }

    json_string = JSONEncoder().encode(request)

    s.send(json_string.encode())
    s.send("\n".encode())

    # read from socket
    total_data = bytes()
    while True:
        data = s.recv(1024)

        if not data:
            break

        total_data += data

        if data[-1] == '\n'.encode()[0]:
            break

    json_response = JSONDecoder().decode(total_data.decode())
    if json_response["type"] == "paste":
        print(json_response["contents"], end='')

    s.close()

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description="Clipboard util. \nUsage: \n\tcat xxx.txt | clip.py copy \n\tclip.py paste > xxx.txt")
    parser.add_argument(dest="command", help="command can be copy and paste")
    args = parser.parse_args()

    if args.command == "copy":
        ss = ''
        while True:
            try:
                ss += input() + '\n'
            except EOFError:
                break

        send(ss)
    elif args.command == "paste":
        request()
    else:
        print("command should be ether copy or paste.")
```
