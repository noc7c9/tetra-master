#!/usr/bin/env python3

import sys
import json
import csv
from colorama import Fore, Style

def parse_args():
    args = sys.argv[1:]
    if '-h' in args or '--help' in args or len(args) == 0:
        print('USAGE:', sys.argv[0], 'FILE...')
        sys.exit(0)
    return args


def load_anydice(fp):
    data = {}
    for row in csv.reader(fp):
        if len(row) < 2: continue
        if type(row[0]) != str: continue
        if ' v ' not in row[0]: continue

        [at, de] = row[0].split(' v ')
        if at not in data:
            data[at] = {}
        data[at][de] = float(row[1])
    return data


def color(string, value):
    if value > 95:
        return Fore.GREEN + string + Fore.RESET
    if value > 80:
        return Fore.BLUE + string + Fore.RESET
    if value < 5:
        return Fore.RED + string + Fore.RESET
    if value < 20:
        return Fore.YELLOW + string + Fore.RESET
    return string


def print_table(name, data):
    def row(att):
        row_data = data[str(att)]
        def datum(def_):
            value = row_data[str(def_)]
            perc = int(value * 100)
            return color('%3d' % perc, perc)
        return tuple(datum(def_) for def_ in range(0, 0x10))

    print("┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓");
    print("┃ %s%69s%s ┃" % (Style.BRIGHT, name, Style.RESET_ALL))
    print("┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫");
    print("┃     D e f e n d e r                                                   ┃")
    print("┃   ┏━━━┳━━━┳━━━┳━━━┳━━━┳━━━┳━━━┳━━━┳━━━┳━━━┳━━━┳━━━┳━━━┳━━━┳━━━┳━━━┳━━━┫");
    print("┃ A ┃   ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃ 4 ┃ 5 ┃ 6 ┃ 7 ┃ 8 ┃ 9 ┃ A ┃ B ┃ C ┃ D ┃ E ┃ F ┃");
    print("┃ t ┣━━━╋━━━╇━━━╇━━━╇━━━╇━━━╇━━━╇━━━╇━━━╇━━━╇━━━╇━━━╇━━━╇━━━╇━━━╇━━━╇━━━┩");
    print("┃ t ┃ 0 ┃%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│" % row(0));
    print("┃ a ┣━━━╉───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤");
    print("┃ k ┃ 1 ┃%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│" % row(1));
    print("┃ c ┣━━━╉───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤");
    print("┃ e ┃ 2 ┃%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│" % row(2));
    print("┃ r ┣━━━╉───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤");
    print("┃   ┃ 3 ┃%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│" % row(3));
    print("┃   ┣━━━╉───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤");
    print("┃   ┃ 4 ┃%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│" % row(4));
    print("┃   ┣━━━╉───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤");
    print("┃   ┃ 5 ┃%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│" % row(5));
    print("┃   ┣━━━╉───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤");
    print("┃   ┃ 6 ┃%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│" % row(6));
    print("┃   ┣━━━╉───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤");
    print("┃   ┃ 7 ┃%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│" % row(7));
    print("┃   ┣━━━╉───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤");
    print("┃   ┃ 8 ┃%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│" % row(8));
    print("┃   ┣━━━╉───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤");
    print("┃   ┃ 9 ┃%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│" % row(9));
    print("┃   ┣━━━╉───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤");
    print("┃   ┃ A ┃%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│" % row(10));
    print("┃   ┣━━━╉───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤");
    print("┃   ┃ B ┃%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│" % row(11));
    print("┃   ┣━━━╉───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤");
    print("┃   ┃ C ┃%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│" % row(12));
    print("┃   ┣━━━╉───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤");
    print("┃   ┃ D ┃%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│" % row(13));
    print("┃   ┣━━━╉───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤");
    print("┃   ┃ E ┃%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│" % row(14));
    print("┃   ┣━━━╉───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤");
    print("┃   ┃ F ┃%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│%s│" % row(15));
    print("┗━━━┻━━━┻━━━┷━━━┷━━━┷━━━┷━━━┷━━━┷━━━┷━━━┷━━━┷━━━┷━━━┷━━━┷━━━┷━━━┷━━━┷━━━┛");


if __name__ == '__main__':
    for file in parse_args():
        with open(file) as fp:
            if file.endswith('.json'):
                data = json.load(fp)
            else:
                data = load_anydice(fp)
            print_table(file, data)
            print()
