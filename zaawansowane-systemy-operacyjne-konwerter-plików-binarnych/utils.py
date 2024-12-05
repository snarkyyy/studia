import re
import pprint
import logging


def setup_logging():
    logging.basicConfig(
        level=logging.INFO, style="{", format="[{levelname}] {message}"
    )


def get_elf_mem_from_path(path):
    return memoryview(bytearray(open(path, "rb").read()))


def pplog(log_fun, obj):
    """Pretty print with logging function."""
    for line in pprint.pformat(obj).split("\n"):
        # https://stackoverflow.com/a/65295800
        line = re.sub(r"=(\d+)", lambda x: "=0x{:x}".format(int(x.group(1))), line)
        log_fun(line)


def get_str(mem: bytes, offset):
    return mem[offset : offset + mem[offset:].find(b"\0")]
