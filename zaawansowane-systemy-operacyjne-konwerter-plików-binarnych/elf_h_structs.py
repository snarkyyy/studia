import struct
import dataclasses

from logging import warning
from utils import get_str, pplog
from dataclasses import dataclass, field

from elf_h_consts import EI_NIDENT
from elf_h_enums import *


def Elf_Ehdr(bits):
    assert bits == 32 or bits == 64
    return Elf32_Ehdr if bits == 32 else Elf64_Ehdr


def Elf_Shdr(bits):
    assert bits == 32 or bits == 64
    return Elf32_Shdr if bits == 32 else Elf64_Shdr


def Elf_Sym(bits):
    assert bits == 32 or bits == 64
    return Elf32_Sym if bits == 32 else Elf64_Sym


def Elf_Rel(bits):
    assert bits == 32 or bits == 64
    return Elf32_Rel if bits == 32 else Elf64_Rel


def Elf_Rela(bits):
    assert bits == 32 or bits == 64
    return Elf32_Rela if bits == 32 else Elf64_Rela


# Used in type annotations to mark extra fields:
NotStructField = object()

# format letters from struct module
unsigned_char = "s"
int32_t = "i"
int64_t = "q"
uint16_t = "H"
uint32_t = "I"
uint64_t = "Q"

Elf32_Half = uint16_t
Elf64_Half = uint16_t
Elf32_Word = uint32_t
Elf32_Sword = int32_t
Elf64_Word = uint32_t
Elf64_Sword = int32_t
Elf32_Xword = uint64_t
Elf32_Sxword = int64_t
Elf64_Xword = uint64_t
Elf64_Sxword = int64_t
Elf32_Addr = uint32_t
Elf64_Addr = uint64_t
Elf32_Off = uint32_t
Elf64_Off = uint64_t
Elf32_Section = uint16_t
Elf64_Section = uint16_t
Elf32_Versym = Elf32_Half
Elf64_Versym = Elf64_Half
Elf32_Conflict = Elf32_Addr


class ElfStruct:
    packer = None

    @classmethod
    def _build_packer_(cls):
        if cls.packer is None:
            flds = dataclasses.fields(cls)
            frmt = "<" + "".join(
                fld.type for fld in flds if fld.type is not NotStructField
            )
            cls.packer = struct.Struct(frmt)

    @classmethod
    def get_packer(cls):
        cls._build_packer_()
        return cls.packer

    @classmethod
    def unpack(cls, mem: memoryview):
        cls._build_packer_()
        mem = mem[: cls.packer.size]
        return cls(*cls.packer.unpack(mem))

    def pack(self):
        self.__class__._build_packer_()
        flds = dataclasses.fields(self)
        flds = [fld for fld in flds if fld.type is not NotStructField]
        vals = tuple(getattr(self, fld.name) for fld in flds)
        return self.packer.pack(*vals)

    @classmethod
    def unpack_array(cls, mem: memoryview, size: int):
        cls._build_packer_()
        single_size = cls.packer.size
        total = cls.packer.size * size
        mem = mem[:total]
        return [
            cls.unpack(mem[i : i + single_size]) for i in range(0, total, single_size)
        ]

    def interpret(self):
        for fld in dataclasses.fields(self):
            if "enum" in fld.metadata:
                try:
                    vars(self)[fld.name] = fld.metadata["enum"](vars(self)[fld.name])
                except ValueError:
                    typename = self.__class__.__name__
                    value = vars(self)[fld.name]
                    enum_name = fld.metadata["enum"].__name__
                    warning(
                        f"{typename}.{fld.name} equal to {value} doesn't match {enum_name} enum."
                    )

    def pack_to_dict(self):
        flds = dataclasses.fields(self)
        return {
            fld.name: vars(self)[fld.name]
            for fld in flds
            if fld.type is not NotStructField
        }

    def __post_init__(self):
        self.interpret()
        self.pack()  # just try packing to see if all values are in bounds and of correct type

    def with_switched_bitness(self):
        other = 32 if self.bits == 64 else 64
        main = self.__class__.main
        return main(other)(**self.pack_to_dict())


# We could parse ident but we can assume it can only take one of those
# two forms and ignore the fields.
IDENT_LINUX64 = b"\x7fELF\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00"
IDENT_LINUX32 = b"\x7fELF\x01\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00"


@dataclass
class Elf32_Ehdr(ElfStruct):
    main = Elf_Ehdr
    bits = 32
    e_ident: str(EI_NIDENT) + unsigned_char = field(
        repr=False
    )  # [EI_NIDENT]; # Magic number and other info
    e_type: Elf32_Half = field(metadata={"enum": ET})  # Object file type
    e_machine: Elf32_Half = field(metadata={"enum": EM})  # Architecture
    e_version: Elf32_Word = field(metadata={"enum": EV})  # Object file version
    e_entry: Elf32_Addr  # Entry point virtual address
    e_phoff: Elf32_Off  # Program header table file offset
    e_shoff: Elf32_Off  # Section header table file offset
    e_flags: Elf32_Word  # Processor-specific flags
    e_ehsize: Elf32_Half  # ELF header size in bytes
    e_phentsize: Elf32_Half  # Program header table entry size
    e_phnum: Elf32_Half  # Program header table entry count
    e_shentsize: Elf32_Half  # Section header table entry size
    e_shnum: Elf32_Half  # Section header table entry count
    e_shstrndx: Elf32_Half  # Section header string table index


@dataclass
class Elf64_Ehdr(ElfStruct):
    main = Elf_Ehdr
    bits = 64
    e_ident: str(EI_NIDENT) + unsigned_char = field(
        repr=False
    )  # [EI_NIDENT]; # Magic number and other info
    e_type: Elf64_Half = field(metadata={"enum": ET})  # Object file type
    e_machine: Elf64_Half = field(metadata={"enum": EM})  # Architecture
    e_version: Elf64_Word = field(metadata={"enum": EV})  # Object file version
    e_entry: Elf64_Addr  # Entry point virtual address
    e_phoff: Elf64_Off  # Program header table file offset
    e_shoff: Elf64_Off  # Section header table file offset
    e_flags: Elf64_Word  # Processor-specific flags
    e_ehsize: Elf64_Half  # ELF header size in bytes
    e_phentsize: Elf64_Half  # Program header table entry size
    e_phnum: Elf64_Half  # Program header table entry count
    e_shentsize: Elf64_Half  # Section header table entry size
    e_shnum: Elf64_Half  # Section header table entry count
    e_shstrndx: Elf64_Half  # Section header string table index


@dataclass
class Elf32_Shdr(ElfStruct):
    main = Elf_Shdr
    bits = 32
    found_name: NotStructField = field(default=None, init=False)
    found_linked_section_name: NotStructField = field(default=None, init=False)
    found_other_linked_section_name: NotStructField = field(default=None, init=False)

    sh_name: Elf32_Word  # Section name (string tbl index)
    sh_type: Elf32_Word = field(metadata={"enum": SHT})  # Section type
    sh_flags: Elf32_Word = field(metadata={"enum": SHF})  # Section flags
    sh_addr: Elf32_Addr  # Section virtual addr at execution
    sh_offset: Elf32_Off  # Section file offset
    sh_size: Elf32_Word  # Section size in bytes
    sh_link: Elf32_Word  # Link to another section
    sh_info: Elf32_Word  # Additional section information
    sh_addralign: Elf32_Word  # Section alignment
    sh_entsize: Elf32_Word  # Entry size if section holds table

    context_shstrtab: NotStructField = field(default=None, repr=False)
    context_section_names: NotStructField = field(default=None, repr=False)

    def interpret(self):
        super().interpret()
        if self.context_shstrtab is not None:
            self.found_name = bytes(get_str(self.context_shstrtab, self.sh_name))
        if self.context_section_names is not None:
            self.found_linked_section_name = bytes(
                self.context_section_names[self.sh_link]
            )
            if self.sh_info < len(self.context_section_names):
                self.found_other_linked_section_name = self.context_section_names[
                    self.sh_info
                ]


@dataclass
class Elf64_Shdr(ElfStruct):
    main = Elf_Shdr
    bits = 64
    found_name: NotStructField = field(default=None, init=False)
    found_linked_section_name: NotStructField = field(default=None, init=False)
    found_other_linked_section_name: NotStructField = field(default=None, init=False)

    sh_name: Elf64_Word  # Section name (string tbl index)
    sh_type: Elf64_Word = field(metadata={"enum": SHT})  # Section type
    sh_flags: Elf64_Xword = field(metadata={"enum": SHF})  # Section flags
    sh_addr: Elf64_Addr  # Section virtual addr at execution
    sh_offset: Elf64_Off  # Section file offset
    sh_size: Elf64_Xword  # Section size in bytes
    sh_link: Elf64_Word  # Link to another section
    sh_info: Elf64_Word  # Additional section information
    sh_addralign: Elf64_Xword  # Section alignment
    sh_entsize: Elf64_Xword  # Entry size if section holds table

    context_shstrtab: NotStructField = field(default=None, repr=False)
    context_section_names: NotStructField = field(default=None, repr=False)

    def interpret(self):
        super().interpret()
        if self.context_shstrtab is not None:
            self.found_name = bytes(get_str(self.context_shstrtab, self.sh_name))
        if self.context_section_names is not None:
            self.found_linked_section_name = bytes(
                self.context_section_names[self.sh_link]
            )
            if self.sh_type == SHT.SHT_RELA:
                self.found_other_linked_section_name = self.context_section_names[
                    self.sh_info
                ]


@dataclass
class Elf32_Sym(ElfStruct):
    main = Elf_Sym
    bits = 32
    found_name: NotStructField = field(default=None, init=False)
    found_section_name: NotStructField = field(default=None, init=False)
    st_name: Elf32_Word  # Symbol name (string tbl index)
    st_value: Elf32_Addr  # Symbol value
    st_size: Elf32_Word  # Symbol size
    st_info: unsigned_char = field(repr=False)  # Symbol type and binding
    st_bind: NotStructField = field(init=False, metadata={"enum": STB})
    st_type: NotStructField = field(init=False, metadata={"enum": STT})
    st_other: unsigned_char = field(repr=False)  # Symbol visibility
    st_visibility: NotStructField = field(init=False, metadata={"enum": STV})
    st_shndx: Elf32_Section  # Section index

    context_strtab: NotStructField = field(default=None, repr=False)
    context_section_names: NotStructField = field(default=None, repr=False)

    def interpret(self):
        if self.context_strtab is not None:
            self.found_name = get_str(self.context_strtab, self.st_name)
        self.st_bind = self.st_info[0] >> 4
        self.st_type = self.st_info[0] & 0xF
        self.st_visibility = self.st_other[0] & 0x03
        if self.context_section_names is not None and self.st_shndx < len(
            self.context_section_names
        ):
            self.found_section_name = self.context_section_names[self.st_shndx]
        super().interpret()

    @classmethod
    def build_info(cls, bind, typ):
        return bytes([(bind << 4) + (typ & 0xF)])


# define ELF32_ST_BIND(val)		(((unsigned char) (val)) >> 4)
# define ELF32_ST_TYPE(val)		((val) & 0xf)
# define ELF32_ST_INFO(bind, type)	(((bind) << 4) + ((type) & 0xf))

# define ELF32_ST_VISIBILITY(o)	((o) & 0x03)


@dataclass
class Elf64_Sym(ElfStruct):
    main = Elf_Sym
    bits = 64
    found_name: NotStructField = field(default=None, init=False)
    found_section_name: NotStructField = field(default=None, init=False)
    st_name: Elf64_Word  # Symbol name (string tbl index)
    st_info: unsigned_char = field(repr=False)  # Symbol type and binding
    st_bind: NotStructField = field(init=False, metadata={"enum": STB})
    st_type: NotStructField = field(init=False, metadata={"enum": STT})
    st_other: unsigned_char = field(repr=False)  # Symbol visibility
    st_visibility: NotStructField = field(init=False, metadata={"enum": STV})
    st_shndx: Elf64_Section  # Section index
    st_value: Elf64_Addr  # Symbol value
    st_size: Elf64_Xword  # Symbol size

    context_strtab: NotStructField = field(default=None, repr=False)
    context_section_names: NotStructField = field(default=None, repr=False)

    def interpret(self):
        if self.context_strtab is not None:
            self.found_name = get_str(self.context_strtab, self.st_name)
        self.st_bind = self.st_info[0] >> 4
        self.st_type = self.st_info[0] & 0xF
        self.st_visibility = self.st_other[0] & 0x03
        if self.context_section_names is not None and self.st_shndx < len(
            self.context_section_names
        ):
            self.found_section_name = self.context_section_names[self.st_shndx]
        super().interpret()

    @classmethod
    def build_info(bind, typ):
        return bytes([(bind << 4) + (typ & 0xF)])


# define ELF64_ST_BIND(val)		ELF32_ST_BIND (val)
# define ELF64_ST_TYPE(val)		ELF32_ST_TYPE (val)
# define ELF64_ST_INFO(bind, type)	ELF32_ST_INFO ((bind), (type))

# define ELF64_ST_VISIBILITY(o)	ELF32_ST_VISIBILITY (o)


@dataclass
class Elf32_Rel(ElfStruct):
    main = Elf_Rel
    bits = 32
    found_symbol_name: NotStructField = field(default=None, init=False)
    r_offset: Elf32_Addr  # Address
    r_info: Elf32_Word  # Relocation type and symbol index
    r_sym: NotStructField = field(init=False)
    r_type: NotStructField = field(init=False)
    r_addend: NotStructField = field(default=None, init=False)

    context_symbol_names: NotStructField = field(default=None, repr=False)
    context_offset_section_content: NotStructField = field(default=None, repr=False)

    def __post_init__(self):
        super().__post_init__()
        self.r_sym = self.r_info >> 8
        self.r_type = R_386(self.r_info & 0xFF)
        if self.context_symbol_names is not None:
            self.found_symbol_name = self.context_symbol_names[self.r_sym]
        if self.context_offset_section_content is not None:
            if self.r_type in [R_386.R_386_32, R_386.R_386_PC32]:
                self.r_addend = struct.unpack(
                    "<i",
                    self.context_offset_section_content[
                        self.r_offset : self.r_offset + 4
                    ],
                )[0]

    @classmethod
    def build_info(cls, *, sym, typ):
        return (sym << 8) + typ


@dataclass
class Elf64_Rel(ElfStruct):
    main = Elf_Rel
    bits = 64
    r_offset: Elf64_Addr  # Address
    r_info: Elf64_Xword  # Relocation type and symbol index


@dataclass
class Elf32_Rela(ElfStruct):
    main = Elf_Rela
    bits = 32
    found_symbol_name: NotStructField = field(default=None, init=False)
    r_offset: Elf32_Addr  # Address
    r_info: Elf32_Word = field(repr=False)  # Relocation type and symbol index
    r_sym: NotStructField = field(init=False)
    r_type: NotStructField = field(init=False)
    r_addend: Elf32_Sword  # Addend

    context_symbol_names: NotStructField = field(default=None, repr=False)
    context_offset_section_content: NotStructField = field(default=None, repr=False)

    def __post_init__(self):
        super().__post_init__()
        self.r_sym = self.r_info >> 8
        self.r_type = R_X86_64(self.r_info & 0xFF)
        if self.context_symbol_names is not None:
            self.found_symbol_name = self.context_symbol_names[self.r_sym]

    @classmethod
    def build_info(cls, *, sym, typ):
        return (sym << 8) + typ

    def change_to_rel(self):
        assert self.r_type == R_386.R_386_32 or self.r_type == R_386.R_386_PC32
        assert self.context_offset_section_content is not None
        self.context_offset_section_content[
            self.r_offset : self.r_offset + 4
        ] = struct.pack("<" + Elf32_Sword, self.r_addend)

        dct = self.pack_to_dict()
        dct.pop("r_addend")
        return Elf_Rel(self.bits)(**dct)


# define ELF32_R_SYM(val)		((val) >> 8)
# define ELF32_R_TYPE(val)		((val) & 0xff)
# define ELF32_R_INFO(sym, type)		(((sym) << 8) + ((type) & 0xff))


@dataclass
class Elf64_Rela(ElfStruct):
    main = Elf_Rela
    bits = 64
    found_symbol_name: NotStructField = field(default=None, init=False)
    r_offset: Elf64_Addr  # Address
    r_info: Elf64_Xword = field(repr=False)  # Relocation type and symbol index
    r_sym: NotStructField = field(init=False)
    r_type: NotStructField = field(init=False)
    r_addend: Elf64_Sxword  # Addend

    context_symbol_names: NotStructField = field(default=None, repr=False)
    context_offset_section_content: NotStructField = field(default=None, repr=False)

    def __post_init__(self):
        super().__post_init__()
        self.r_sym = self.r_info >> 32
        self.r_type = R_X86_64(self.r_info & 0xFFFFFFFF)
        if self.context_symbol_names is not None:
            self.found_symbol_name = self.context_symbol_names[self.r_sym]

    @classmethod
    def build_info(cls, *, sym, typ):
        return (sym << 32) + typ

    def with_switched_bitness(self):
        other = 32 if self.bits == 64 else 64
        assert other == 32, "Change from 32 to 64 not implemented"
        main = self.__class__.main

        if self.r_type == R_X86_64.R_X86_64_32 or self.r_type == R_X86_64.R_X86_64_32S:
            new_r_type = R_386.R_386_32
        elif (
            self.r_type == R_X86_64.R_X86_64_PLT32
            or self.r_type == R_X86_64.R_X86_64_PC32
        ):
            new_r_type = R_386.R_386_PC32

        dct = self.pack_to_dict()
        dct["r_info"] = main(other).build_info(sym=self.r_sym, typ=new_r_type)
        return main(other)(**dct)


# define ELF64_R_SYM(i)			((i) >> 32)
# define ELF64_R_TYPE(i)			((i) & 0xffffffff)
# define ELF64_R_INFO(sym,type)		((((Elf64_Xword) (sym)) << 32) + (type))
