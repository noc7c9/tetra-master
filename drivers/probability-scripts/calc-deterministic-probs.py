#!/usr/bin/env python3

import json
from collections import defaultdict

data = defaultdict(dict)
for at in range(0, 0xF + 1):
    for de in range(0, 0xF + 1):
        data[at][de] = 1.0 if at > de else 0.0

print(json.dumps(data))
