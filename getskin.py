#!/usr/bin/env python3

import sys
import shutil

import requests

mc_skin_url = "http://s3.amazonaws.com/MinecraftSkins/{username}.png"

def main():
    if len(sys.argv) < 2:
        print('Please provide a username', file=sys.stderr)
        sys.exit(1)

    r = requests.get(mc_skin_url.format(username=sys.argv[1]), stream=True)
    if r.status_code != 200:
        print('Failed to download skin', file=sys.stderr)
        sys.exit(1)

    with open("{username}.png".format(username=sys.argv[1]), 'wb') as f:
        shutil.copyfileobj(r.raw, f)

if __name__ == '__main__':
    main()
