
## Description
A simple clipboard server implemented in rust. the client was implemented in python


## Usage

1. clone this repo, cd into it. `cargo install --path .`
2. ssh to your machine, with "Remote Forwarding" argument `-R`. `ssh -fNR 33304:127.0.0.1:33304 remote_user@remote_host`
3. save the following script into your `~/.local/bin/clip.py`, don't forget to add `${HOME}/.local/bin` into your `$PATH`.

Bang! you can copy with `cat xxx.txt | clip.py copy`, and paste with `clip.py paste` now!

``` python
#!/usr/bin/env python3

import socket
import argparse
from json import JSONEncoder, JSONDecoder
from sys import stderr

def send(ss: str):
    s = socket.socket()
    s.connect(('127.0.0.1',33304))

    if ss.endswith('\n'):
        ss = ss[:-1]

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

## Advance settings
- Well, for Emacs Users, into your `.emacs.d` (I recommend this, because i love it.)
```lisp
(defun my/copy-handler (text)
    (setq my/copy-process (make-process :name "my/copy"
                                        :buffer nil
                                        :command '("~/.local/bin/clip.py" "copy")
                                        :connection-type 'pipe))
    (process-send-string my/copy-process text)
    (process-send-eof my/copy-process))
  (defun my/paste-handler ()
    (if (and my/copy-process (process-live-p my/copy-process))
        nil ; should return nil if we're the current paste owner
      (shell-command-to-string "~/.local/bin/clip.py paste | tr -d '\r'")))
  (setq interprogram-cut-function 'my/copy-handler
        interprogram-paste-function 'my/paste-handler)
```
- Okay, for Neovim users, into your `init.lua`
```lua
vim.g.clipboard = {
  name = "remote",
  copy = {
    ["+"] = "clip.py copy",
    ["*"] = "clip.py copy",
  },
  paste = {
    ["+"] = "clip.py paste",
    ["*"] = "clip.py paste",
  },
}
```
