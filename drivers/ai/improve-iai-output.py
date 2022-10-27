#!/usr/bin/env python3

import fileinput
from itertools import chain

results = {}
curr = None
for line in fileinput.input():
    print(line, end='') # passthrough

    line = line.strip()
    if line.startswith('Instructions:'):
        results[curr]['instructions'] = int(line.split()[1])
    elif line.startswith('L1 Accesses:'):
        results[curr]['l1-accesses'] = int(line.split()[2])
    elif line.startswith('L2 Accesses:'):
        results[curr]['l2-accesses'] = int(line.split()[2])
    elif line.startswith('RAM Accesses:'):
        results[curr]['ram-accesses'] = int(line.split()[2])
    elif line.startswith('Estimated Cycles:'):
        results[curr]['estimated-cycles'] = int(line.split()[2])
    elif line != '':
        curr = line
        results[curr] = {}

first, *rest = results.keys()
first = results[first]

print('--- Deltas ---')
for name in rest:
    res = results[name]
    print(name)
    print('  Instructions:', res['instructions'] - first['instructions'])
    print('  L1 Accesses:', res['l1-accesses'] - first['l1-accesses'])
    print('  L2 Accesses:', res['l2-accesses'] - first['l2-accesses'])
    print('  RAM Accesses:', res['ram-accesses'] - first['ram-accesses'])
    print('  Estimated Cycles:', res['estimated-cycles'] - first['estimated-cycles'])
    print()
