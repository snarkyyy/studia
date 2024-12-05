from typing import Any, List
import collections
from logging import warning, debug, info
from dataclasses import dataclass, replace, field

from elf_h import *
from stubs import Stub, get_stub_64_to_32, get_stub_32_to_64
from utils import pplog

MemoryRegion = collections.namedtuple(
    "MemoryRegion",
    """
    name
    start_fun
    length
    alignment
    move_fun
    pack_fun
""",
)


@dataclass
class Section:
    section_type_to_enttype = {
        SHT.SHT_RELA: Elf_Rela,
        SHT.SHT_SYMTAB: Elf_Sym,
        SHT.SHT_REL: Elf_Rel,
    }

    header: Elf_Shdr

    context_elf_mem: memoryview = field(default=None, repr=False)
    context_sections: Any = field(default=None, repr=False)
    context_shstrtab: bytes = field(default=None, repr=False)

    content: Any = field(default=None)

    def interpret_content_from_context(self):
        assert self.has_content()

        bits = self.header.bits
        offset = self.header.sh_offset
        size = self.header.sh_size
        entsize = self.header.sh_entsize
        typ = self.header.sh_type

        if typ == SHT.SHT_STRTAB:  # we want bytearray so it will print itself out
            self.content = bytearray(self.context_elf_mem[offset : offset + size])
        else:  # we want memory so it won't print itself out and modification won't resize
            self.content = memoryview(
                bytearray(self.context_elf_mem[offset : offset + size])
            )

        if typ in self.section_type_to_enttype:
            enttype = self.section_type_to_enttype[typ](bits)
            self.content = enttype.unpack_array(self.content, size // entsize)

    def interpret_symbol_names(self):
        assert self.header.sh_type == SHT.SHT_SYMTAB
        assert self.context_sections is not None

        strtab = bytes(self.context_sections[self.header.sh_link].content)
        self.content = [replace(sym, context_strtab=strtab) for sym in self.content]

        arbitrary_name = self.context_sections[0].header.found_name
        if arbitrary_name is not None:
            section_names = [s.header.found_name for s in self.context_sections]
            self.content = [
                replace(sym, context_section_names=section_names)
                for sym in self.content
            ]

    def give_shstrtab_to_header(self):
        assert self.context_shstrtab is not None
        self.header = replace(self.header, context_shstrtab=self.context_shstrtab)

    def give_section_names_to_header(self):
        assert (
            self.context_sections[0].header.found_name is not None
            and self.context_sections is not None
        )
        section_names = [s.header.found_name for s in self.context_sections]
        self.header = replace(self.header, context_section_names=section_names)

    def give_symbol_names_to_relocations(self):
        assert (
            self.header.sh_type in [SHT.SHT_RELA, SHT.SHT_REL]
            and self.context_sections is not None
        )
        symbol_section = self.context_sections[self.header.sh_link]
        symbols = symbol_section.content
        names = [
            s.found_section_name if s.st_type == STT.STT_SECTION else s.found_name
            for s in symbols
        ]
        self.content = [
            replace(rel, context_symbol_names=names) for rel in self.content
        ]

    def give_offset_section_content_to_relocations(self):
        assert (
            self.header.sh_type in [SHT.SHT_RELA, SHT.SHT_REL]
            and self.context_sections is not None
        )
        offset_section_content = self.context_sections[self.header.sh_info].content
        self.content = [
            replace(rel, context_offset_section_content=offset_section_content)
            for rel in self.content
        ]

    def __post_init__(self):

        bits = self.header.bits
        offset = self.header.sh_offset
        size = self.header.sh_size
        entsize = self.header.sh_entsize
        typ = self.header.sh_type

        if self.context_shstrtab is not None:
            self.give_shstrtab_to_header()

        if self.context_elf_mem is not None and self.has_content():
            self.interpret_content_from_context()

        if self.context_sections is not None:

            if typ == SHT.SHT_SYMTAB:
                self.interpret_symbol_names()

            if self.context_sections[0].header.found_name is not None:
                self.give_section_names_to_header()

            if typ in [SHT.SHT_RELA, SHT.SHT_REL]:
                self.give_symbol_names_to_relocations()
                self.give_offset_section_content_to_relocations()

    def has_content(self):
        return self.header.sh_offset != 0 and self.header.sh_size != 0

    def pack_content(self):
        assert self.has_content()
        if self.header.sh_type not in self.section_type_to_enttype:
            packed = self.content
        else:
            packed = b"".join(elem.pack() for elem in self.content)
        assert len(packed) == self.header.sh_size
        return packed

    def get_memory_region_for_content(self):
        assert self.has_content()

        def move(addr):
            assert (addr % self.header.sh_addralign) == 0
            self.header.sh_offset = addr

        return MemoryRegion(
            name=self.header.found_name.decode(),
            start_fun=lambda: self.header.sh_offset,
            length=len(self.pack_content()),
            alignment=self.header.sh_addralign,
            move_fun=move,
            pack_fun=lambda: self.pack_content(),
        )

    def with_switched_bitness(self):
        switched_header = self.header.with_switched_bitness()
        switched_content = self.content
        if self.header.sh_type in self.section_type_to_enttype:
            switched_content = [elem.with_switched_bitness() for elem in self.content]
            switched_header.sh_entsize = switched_content[0].get_packer().size
            switched_header.sh_size = len(switched_content) * switched_header.sh_entsize
        return Section(header=switched_header, content=switched_content)

    def change_type_from_rela_to_rel(self):
        assert self.header.sh_type == SHT.SHT_RELA
        self.header.sh_entsize = Elf_Rel(self.header.bits).get_packer().size
        self.header.sh_size = len(self.content) * self.header.sh_entsize
        self.header.sh_type = SHT.SHT_REL
        self.content = [rela.change_to_rel() for rela in self.content]

    def add_str(self, s):
        assert self.header.sh_type == SHT.SHT_STRTAB
        self.content.extend(s + b"\0")
        offset = self.header.sh_size
        self.header.sh_size = len(self.content)
        return offset

    def append_ent(self, ent):
        assert self.header.sh_type in self.section_type_to_enttype
        self.content.append(ent)
        self.header.sh_size += self.header.sh_entsize
        return len(self.content) - 1

    def append_cont(self, data):
        self.content.extend(data)
        self.header.sh_size += len(data)
        return len(self.content) - 1


@dataclass
class RelElf:
    ehdr: Elf_Ehdr = None
    sections: List[Section] = None

    context_elf_mem: memoryview = field(default=None, repr=False)

    def __post_init__(self):
        mem = self.context_elf_mem
        if mem is not None:
            bits = 32 if mem[:EI_NIDENT] == IDENT_LINUX32 else 64
            self.ehdr = Elf_Ehdr(bits).unpack(mem)
            assert self.ehdr.e_type == ET.ET_REL
            shdrs = Elf_Shdr(bits).unpack_array(
                mem[self.ehdr.e_shoff :], self.ehdr.e_shnum
            )
            self.sections = [
                Section(header=shdr, context_elf_mem=mem) for shdr in shdrs
            ]
        self.interpret()

    def interpret(self):
        shstrtab = self.sections[self.ehdr.e_shstrndx].content
        self.sections = [replace(s, context_shstrtab=shstrtab) for s in self.sections]
        # Reinitialize twice to propagate extra information thurough structures.
        for _ in range(2):
            self.sections = [
                replace(s, context_sections=self.sections) for s in self.sections
            ]

    def with_switched_bitness(self):
        other = 32 if self.ehdr.bits == 64 else 64
        ehdr = self.ehdr.with_switched_bitness()
        ehdr.e_ident = IDENT_LINUX32 if other == 32 else IDENT_LINUX64
        ehdr.e_machine = EM.EM_386 if other == 32 else EM.EM_X86_64
        ehdr.e_ehsize = Elf_Ehdr(other).get_packer().size
        ehdr.e_shentsize = Elf_Shdr(other).get_packer().size

        sections = [s.with_switched_bitness() for s in self.sections]
        return RelElf(ehdr=ehdr, sections=sections)

    def change_relas_to_rels(self, correct_names=True):
        for s in self.sections:
            if s.header.sh_type == SHT.SHT_RELA:
                s.change_type_from_rela_to_rel()

        if correct_names:
            # Change section names from .rela.??? to .rel.???:
            # Assumes that for every .rela.??? there is
            # section ??? with sh_name to change
            shstrtab_section = self.sections[self.ehdr.e_shstrndx]
            shstrtab = bytearray(shstrtab_section.content)
            shstrtab = list(shstrtab.split(b"\0"))
            for shstr in shstrtab:
                if shstr.startswith(b".rela"):
                    prog_section_name = shstr[len(b".rela") :]
                    shstr[: len(".rela")] = b".rel"
                    shstr.extend(b"\0")
                    for s in self.sections:
                        if s.header.found_name == prog_section_name:
                            s.header.sh_name -= 1

            shstrtab = bytearray(b"\0".join(shstrtab))
            shstrtab_section.content = shstrtab

        self.interpret()

    def get_memory_region_for_shdrs(self):
        alignment = 8

        def move(addr):
            assert (addr % alignment) == 0
            self.ehdr.e_shoff = addr

        def pack_shdrs():
            return b"".join(shdr.pack() for shdr in shdrs)

        shdrs = [s.header for s in self.sections]

        return MemoryRegion(
            name="section headers",
            start_fun=lambda: self.ehdr.e_shoff,
            length=len(pack_shdrs()),
            alignment=8,
            move_fun=move,
            pack_fun=lambda: b"".join(shdr.pack() for shdr in shdrs),
        )

    def get_memory_region_for_header(self):
        def move(addr):
            assert addr == 0

        return MemoryRegion(
            name="header",
            start_fun=lambda: 0,
            length=self.ehdr.get_packer().size,
            alignment=1,
            move_fun=move,
            pack_fun=lambda: self.ehdr.pack(),
        )

    def append_section(self, s):
        self.sections.append(s)
        self.ehdr.e_shnum += 1
        return len(self.sections) - 1

    def add_thunks(self, sigs):
        assert self.ehdr.bits == 32

        old_rels = [s for s in self.sections if s.header.sh_type == SHT.SHT_REL]

        sigs_by_name = {sig.fun_name.encode(): sig for sig in sigs}
        shstrtab_section = self.sections[self.ehdr.e_shstrndx]
        symtab_sections = [
            (i, s)
            for i, s in enumerate(self.sections)
            if s.header.sh_type == SHT.SHT_SYMTAB
        ]

        if len(symtab_sections) == 0:
            return

        assert len(symtab_sections) == 1
        symtab_index, symtab_section = symtab_sections[0]
        strtab_section = self.sections[symtab_section.header.sh_link]

        def create_progbits_section(name, flags):
            return Section(
                header=Elf32_Shdr(
                    sh_name=shstrtab_section.add_str(name),
                    sh_type=SHT.SHT_PROGBITS,
                    sh_flags=flags,  # SHF.SHF_EXECINSTR | SHF.SHF_ALLOC,
                    sh_addr=0x0,
                    sh_offset=self.ehdr.e_shoff - 1,
                    sh_size=0x0,
                    sh_link=0x0,
                    sh_info=0x0,
                    sh_addralign=0x8,
                    sh_entsize=0x0,
                ),
                content=bytearray(),
            )

        exec_flags = SHF.SHF_EXECINSTR | SHF.SHF_ALLOC
        text_thunkin = create_progbits_section(b".text.thunkin", exec_flags)
        text_thunkout = create_progbits_section(b".text.thunkout", exec_flags)

        text_thunkin_idx = self.append_section(text_thunkin)
        text_thunkout_idx = self.append_section(text_thunkout)

        rodata_flags = SHF.SHF_ALLOC
        rodata_thunkin = create_progbits_section(b".rodata.thunkin", rodata_flags)
        rodata_thunkout = create_progbits_section(b".rodata.thunkout", rodata_flags)

        rodata_thunkin_idx = self.append_section(rodata_thunkin)
        rodata_thunkout_idx = self.append_section(rodata_thunkout)

        def create_rela_section(name, progbits_idx):
            return Section(
                header=Elf32_Shdr(
                    sh_name=shstrtab_section.add_str(name),
                    sh_type=SHT.SHT_RELA,
                    sh_flags=0,
                    sh_addr=0x0,
                    sh_offset=self.ehdr.e_shoff - 1,
                    sh_size=0x0,
                    sh_link=symtab_index,
                    sh_info=progbits_idx,
                    sh_addralign=0x8,
                    sh_entsize=Elf_Rela(32).get_packer().size,
                ),
                content=[],
            )

        # those will be changed to rels later...
        text_thunkin_rela = create_rela_section(b".rel.text.thunkin", text_thunkin_idx)
        text_thunkout_rela = create_rela_section(
            b".rel.text.thunkout", text_thunkout_idx
        )

        self.sections.extend([text_thunkin_rela, text_thunkout_rela])
        self.ehdr.e_shnum += 2

        rodata_thunkin_rela = create_rela_section(
            b".rel.rodata.thunkin", rodata_thunkin_idx
        )
        rodata_thunkout_rela = create_rela_section(
            b".rel.rodata.thunkout", rodata_thunkout_idx
        )

        self.sections.extend([rodata_thunkin_rela, rodata_thunkout_rela])
        self.ehdr.e_shnum += 2

        def create_section_symbol(shndx):
            return Elf32_Sym(
                st_name=0x0,
                st_value=0x0,
                st_size=0x0,
                st_info=Elf32_Sym.build_info(bind=STB.STB_LOCAL, typ=STT.STT_SECTION),
                st_other=bytes([STV.STV_DEFAULT]),
                st_shndx=shndx,
            )

        text_thunkin_symbol_idx = symtab_section.append_ent(
            create_section_symbol(text_thunkin_idx)
        )

        text_thunkout_symbol_idx = symtab_section.append_ent(
            create_section_symbol(text_thunkout_idx)
        )

        rodata_thunkin_symbol_idx = symtab_section.append_ent(
            create_section_symbol(rodata_thunkin_idx)
        )

        rodata_thunkout_symbol_idx = symtab_section.append_ent(
            create_section_symbol(rodata_thunkout_idx)
        )

        syms_64_to_32 = []
        syms_32_to_64 = []

        org_len = len(symtab_section.content)
        for idx, sym in enumerate(symtab_section.content):
            if idx >= org_len:
                break
            if sym.st_bind == STB.STB_GLOBAL:
                sym.st_info = Elf_Sym(sym.bits).build_info(
                    bind=STB.STB_LOCAL, typ=sym.st_type
                )
                sym.interpret()
                if sym.st_type == STT.STT_FUNC:
                    syms_32_to_64.append((idx, sym))
                elif sym.st_type == STT.STT_NOTYPE:
                    syms_64_to_32.append((idx, sym))
                else:
                    symtab_section.append_ent(
                        replace(
                            sym,
                            st_info=Elf_Sym(sym.bits).build_info(
                                bind=STB.STB_GLOBAL, typ=sym.st_type
                            ),
                        )
                    )

        symtab_section.header.sh_info = org_len

        for idx, sym in syms_32_to_64:
            sig = sigs_by_name[sym.found_name]
            stub = get_stub_32_to_64(sig)
            size_so_far = len(text_thunkin.content)
            new_sym = replace(
                sym,
                st_shndx=text_thunkin_idx,
                st_size=len(stub.machine_code),
                st_value=size_so_far,
                st_info=Elf_Sym(32).build_info(bind=STB.STB_GLOBAL, typ=STT.STT_FUNC),
            )
            new_sym_idx = symtab_section.append_ent(new_sym)
            text_thunkin_rela.append_ent(
                Elf_Rela(32)(
                    r_offset=size_so_far + stub.first_jump_relo_offset,
                    r_info=Elf_Rela(32).build_info(
                        sym=rodata_thunkin_symbol_idx, typ=R_386.R_386_32
                    ),
                    r_addend=len(rodata_thunkin.content),
                )
            )
            rodata_thunkin.append_cont(b"\0" * 4 + b"\x33\x00\x00\x00")
            text_thunkin_rela.append_ent(
                Elf_Rela(32)(
                    r_offset=size_so_far + stub.call_relo_offset,
                    r_info=Elf_Rela(32).build_info(sym=idx, typ=R_386.R_386_PC32),
                    r_addend=-4,
                )
            )
            text_thunkin_rela.append_ent(
                Elf_Rela(32)(
                    r_offset=size_so_far + stub.second_jump_relo_offset,
                    r_info=Elf_Rela(32).build_info(
                        sym=rodata_thunkin_symbol_idx, typ=R_386.R_386_32
                    ),
                    r_addend=len(rodata_thunkin.content),
                )
            )
            rodata_thunkin.append_cont(b"\0" * 4 + b"\x23\x00\x00\x00")
            text_thunkin.append_cont(stub.machine_code)

            rodata_thunkin_rela.append_ent(
                Elf_Rela(32)(
                    r_offset=len(rodata_thunkin.content) - 0x10,
                    r_info=Elf_Rela(32).build_info(
                        typ=R_386.R_386_32, sym=text_thunkin_symbol_idx
                    ),
                    r_addend=stub.first_jump_relo_offset + 0x4,
                )
            )
            rodata_thunkin_rela.append_ent(
                Elf_Rela(32)(
                    r_offset=len(rodata_thunkin.content) - 0x08,
                    r_info=Elf_Rela(32).build_info(
                        typ=R_386.R_386_32, sym=text_thunkin_symbol_idx
                    ),
                    r_addend=stub.second_jump_relo_offset + 0x4,
                )
            )

        for idx, sym in syms_64_to_32:
            sig = sigs_by_name[sym.found_name]
            stub = get_stub_64_to_32(sig)
            size_so_far = len(text_thunkout.content)
            new_sym = replace(
                sym,
                st_info=Elf_Sym(32).build_info(bind=STB.STB_GLOBAL, typ=STT.STT_NOTYPE),
            )

            sym.st_value = size_so_far
            sym.st_size = len(stub.machine_code)
            sym.st_info = Elf_Sym(32).build_info(bind=STB.STB_LOCAL, typ=STT.STT_NOTYPE)
            sym.st_shndx = text_thunkout_idx
            sym.interpret()

            new_sym_idx = symtab_section.append_ent(new_sym)

            text_thunkout_rela.append_ent(
                Elf_Rela(32)(
                    r_offset=size_so_far + stub.first_jump_relo_offset,
                    r_info=Elf_Rela(32).build_info(
                        sym=rodata_thunkout_symbol_idx, typ=R_386.R_386_32
                    ),
                    r_addend=len(rodata_thunkout.content),
                )
            )
            rodata_thunkout.append_cont(b"\0" * 4 + b"\x23\x00\x00\x00")
            text_thunkout_rela.append_ent(
                Elf_Rela(32)(
                    r_offset=size_so_far + stub.call_relo_offset,
                    r_info=Elf_Rela(32).build_info(
                        sym=new_sym_idx, typ=R_386.R_386_PC32
                    ),
                    r_addend=-4,
                )
            )
            text_thunkout_rela.append_ent(
                Elf_Rela(32)(
                    r_offset=size_so_far + stub.second_jump_relo_offset,
                    r_info=Elf_Rela(32).build_info(
                        sym=rodata_thunkout_symbol_idx, typ=R_386.R_386_32
                    ),
                    r_addend=len(rodata_thunkout.content),
                )
            )
            rodata_thunkout.append_cont(b"\0" * 4 + b"\x33\x00\x00\x00")

            rodata_thunkout_rela.append_ent(
                Elf_Rela(32)(
                    r_offset=len(rodata_thunkout.content) - 0x10,
                    r_info=Elf_Rela(32).build_info(
                        typ=R_386.R_386_32, sym=text_thunkout_symbol_idx
                    ),
                    r_addend=len(text_thunkout.content)
                    + stub.first_jump_relo_offset
                    + 0x4,
                )
            )
            rodata_thunkout_rela.append_ent(
                Elf_Rela(32)(
                    r_offset=len(rodata_thunkout.content) - 0x08,
                    r_info=Elf_Rela(32).build_info(
                        typ=R_386.R_386_32, sym=text_thunkout_symbol_idx
                    ),
                    r_addend=len(text_thunkout.content)
                    + stub.second_jump_relo_offset
                    + 0x4,
                )
            )

            text_thunkout.append_cont(stub.machine_code)

        self.interpret()

        rodata_thunkin.content = memoryview(rodata_thunkin.content)
        rodata_thunkout.content = memoryview(rodata_thunkout.content)
        self.change_relas_to_rels(correct_names=False)

    def write_to_file(self, path):
        bits = self.ehdr.bits
        assert self.ehdr.e_type == ET.ET_REL
        assert self.ehdr.e_ident == (IDENT_LINUX32 if bits == 32 else IDENT_LINUX64)
        for section in self.sections:
            assert section.header.bits == bits
        regions = []

        def add_region(region):
            regions.append(region)

        add_region(self.get_memory_region_for_header())

        for section in self.sections:
            if section.has_content():
                add_region(section.get_memory_region_for_content())

        add_region(self.get_memory_region_for_shdrs())

        regions.sort(key=lambda x: x.start_fun())

        end_ptr = 0
        for r in regions:
            if r.start_fun() < end_ptr:
                new_start = end_ptr + (-end_ptr % r.alignment)
                debug(
                    f'moving "{r.name}" region from {hex(r.start_fun())} to {hex(new_start)}'
                )
                r.move_fun(end_ptr + (-end_ptr % r.alignment))
            end_ptr = r.start_fun() + r.length

        size = end_ptr

        elf = bytearray(size)

        for r in regions:
            data = r.pack_fun()
            debug(
                f'writing region "{r.name}" from {hex(r.start_fun())} to {hex(r.start_fun() + len(data) - 1)}'
            )
            elf[r.start_fun() : r.start_fun() + len(data)] = data

        with open(path, "wb") as f:
            f.write(elf)
