import collections

from pwnlib.asm import asm, disasm

from signatures import Signature

Stub = collections.namedtuple(
    "Stub",
    """
    machine_code 
    first_jump_relo_offset 
    call_relo_offset
    second_jump_relo_offset
""",
)

ljmp = bytes(b"\xff\x2c\x25\x00\x00\x00\x00")
ljmp_relo_position = 3

call = bytes(b"\xe8\x00\x00\x00\x00")
call_relo_position = 1

arg_pos_to_reg = [
    {8: "rdi", 4: "edi"},
    {8: "rsi", 4: "esi"},
    {8: "rdx", 4: "edx"},
    {8: "rcx", 4: "ecx"},
    {8: "r8", 4: "r8d"},
    {8: "r9", 4: "r9d"},
]

type_to_size = {
    "int": 4,
    "uint": 4,
    "long": 4,
    "ulong": 4,
    "longlong": 8,
    "ulonglong": 8,
    "ptr": 4,
}


def get_stub_64_to_32(sig):

    start = """
        push rbx;
        push rbp;
        push r12;
        push r13;
        push r14;
        push r15;
    """

    start_rsp_addend = 6 * 8

    args_size = sum(type_to_size[typ] for typ in sig.argument_types)

    what_to_sub = args_size + ((8 - args_size) % 16)

    args_to_stack = [f"sub rsp, {what_to_sub}"]

    offset = 0
    for pos, typ in enumerate(sig.argument_types):
        args_to_stack.append(
            f"mov [rsp + {offset}], {arg_pos_to_reg[pos][type_to_size[typ]]}"
        )
        offset += type_to_size[typ]

    args_to_stack = "\n".join(args_to_stack)

    part32 = """
        push 0x2b
        pop ds
        push 0x2b
        pop es
    """

    ret_part = ""
    if sig.return_type == "longlong":
        ret_part = """
            mov eax, eax
            shl rdx, 32
            or rax, rdx
        """
    elif sig.return_type == "int":
        ret_part = """
            mov eax, eax
        """
    elif sig.return_type == "void":
        pass
    else:
        assert False, f"{sig.return_type}"

    clear_stack = f"""
        add rsp, {what_to_sub}
    """

    end = """
        pop r15
        pop r14
        pop r13
        pop r12
        pop rbp
        pop rbx
        ret
    """

    front = start + args_to_stack

    back = ret_part + clear_stack + end

    front = asm(front, arch="amd64", os="linux")

    part32 = asm(part32, arch="i386", os="linux")

    back = asm(back, arch="amd64", os="linux")

    machine_code = front + ljmp + part32 + call + ljmp + back
    first_jump_relo_offset = len(front) + ljmp_relo_position
    call_relo_offset = len(front) + len(ljmp) + len(part32) + call_relo_position
    second_jump_relo_offset = (
        len(front) + len(ljmp) + len(part32) + len(call) + ljmp_relo_position
    )

    # print(disasm(front + ljmp, arch='amd64', os='linux'))
    # print(disasm(part32 + call, arch='i386', os='linux'))
    # print(disasm(ljmp + back, arch='amd64', os='linux'))

    for off in [first_jump_relo_offset, call_relo_offset, second_jump_relo_offset]:
        # print("hello")
        assert (
            machine_code[off : off + 4] == b"\0\0\0\0"
        ), f"{machine_code[off: off + 4].hex()}"

    return Stub(
        machine_code=machine_code,
        first_jump_relo_offset=first_jump_relo_offset,
        call_relo_offset=call_relo_offset,
        second_jump_relo_offset=second_jump_relo_offset,
    )


def get_stub_32_to_64(sig):

    start = """
        push   edi
        push   esi
    """

    args_size = sum(type_to_size[typ] for typ in sig.argument_types)

    # warning(f"{sig}")
    # warning(f"{sig.fun_name}: args_size={args_size}")
    what_to_sub = 4

    stack_sub = f"""
        sub esp, {what_to_sub}
    """

    stack_to_regs = []

    offset = 16
    for pos, typ in enumerate(sig.argument_types):
        if typ == "long":
            stack_to_regs.append(
                f"movsx {arg_pos_to_reg[pos][8]}, dword [rsp + {offset}]"
            )
        else:
            stack_to_regs.append(
                f"mov {arg_pos_to_reg[pos][type_to_size[typ]]}, [rsp + {offset}]"
            )
        offset += type_to_size[typ]

    stack_to_regs = "\n".join(stack_to_regs)

    part64 = """
    """

    ret_part = """
        mov rdx, rax
        shr rdx, 32
    """

    clear_stack = f"""
        add esp, {what_to_sub}
    """

    end = """
        pop esi
        pop edi
        ret
    """

    front = start + stack_sub

    front_part64 = stack_to_regs + part64

    back_part64 = ret_part

    back = clear_stack + end

    front = asm(front, arch="i386", os="linux")

    front_part64 = asm(front_part64, arch="amd64", os="linux")
    back_part64 = asm(back_part64, arch="amd64", os="linux")

    back = asm(back, arch="i386", os="linux")

    machine_code = front + ljmp + front_part64 + call + back_part64 + ljmp + back
    first_jump_relo_offset = len(front) + ljmp_relo_position
    call_relo_offset = len(front) + len(ljmp) + len(front_part64) + call_relo_position
    second_jump_relo_offset = (
        len(front)
        + len(ljmp)
        + len(front_part64)
        + len(call)
        + len(back_part64)
        + ljmp_relo_position
    )

    # print(disasm(front, arch='i386', os='linux'))
    # print(disasm(ljmp + part64 + call + ljmp, arch='amd64', os='linux'))
    # print(disasm(back, arch='i386', os='linux'))

    for off in [first_jump_relo_offset, call_relo_offset, second_jump_relo_offset]:
        assert machine_code[off : off + 4] == b"\0\0\0\0"

    return Stub(
        machine_code=machine_code,
        first_jump_relo_offset=first_jump_relo_offset,
        call_relo_offset=call_relo_offset,
        second_jump_relo_offset=second_jump_relo_offset,
    )


get_stub_64_to_32(
    Signature(
        return_type="longlong",
        fun_name="dupa",
        argument_types=["ptr", "int", "longlong"],
    )
)
get_stub_32_to_64(
    Signature(
        return_type="longlong",
        fun_name="dupa",
        argument_types=["ptr", "int", "longlong"],
    )
)

# print(asm('nop'))

# front, part32, back = get_stub_64_to_32("longlong", ["ptr", "int", "longlong"])

# print(disasm(front, arch='amd64', os='linux'))
# print(disasm(part32, arch='i386', os='linux'))
# print(disasm(back, arch='amd64', os='linux'))

# front, part64, back = get_stub_32_to_64("longlong", ["ptr", "int", "longlong"])

# print(disasm(front, arch='i386', os='linux'))
# print(disasm(part64, arch='amd64', os='linux'))
# print(disasm(back, arch='i386', os='linux'))

# 11:   ff 2c 25 00 00 00 00    jmp    FWORD PTR [eiz*1+0x0]
# 18:   6a 2b                   push   0x2b
# 1a:   1f                      pop    ds
# 1b:   6a 2b                   push   0x2b
# 1d:   07                      pop    es
# 1e:   e8 fc ff ff ff          call   1f <pnum+0x1f>
# 23:   ff 2c 25 08 00 00 00    jmp    FWORD PTR [eiz*1+0x8]
