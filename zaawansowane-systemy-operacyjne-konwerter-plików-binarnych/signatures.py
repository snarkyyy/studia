import collections

Signature = collections.namedtuple("Signature", "return_type fun_name argument_types")


def read_signatures_from_file(path):
    with open(path) as f:
        sigs = f.read().split("\n")
    sigs = [sig for sig in sigs if len(sig) > 0]
    sigs = [list(sig.split()) for sig in sigs]
    sigs = [
        Signature(return_type=sig[1], fun_name=sig[0], argument_types=sig[2:])
        for sig in sigs
    ]
    return sigs
