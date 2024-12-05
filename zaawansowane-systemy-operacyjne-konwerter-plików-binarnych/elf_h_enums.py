from enum import IntEnum, IntFlag


class ET(IntEnum):
    ET_REL = 1  # Relocatable file */


class EM(IntEnum):
    EM_386 = 3  # Intel 80386 */
    EM_X86_64 = 62  # AMD x86-64 architecture */


class EV(IntEnum):
    EV_CURRENT = 1  # Current version */


class SHT(IntEnum):
    SHT_NULL = 0  # Section header table entry unused */
    SHT_PROGBITS = 1  # Program data */
    SHT_SYMTAB = 2  # Symbol table */
    SHT_STRTAB = 3  # String table */
    SHT_RELA = 4  # Relocation entries with addends */
    SHT_HASH = 5  # Symbol hash table */
    SHT_DYNAMIC = 6  # Dynamic linking information */
    SHT_NOTE = 7  # Notes */
    SHT_NOBITS = 8  # Program space with no data (bss) */
    SHT_REL = 9  # Relocation entries, no addends */
    SHT_SHLIB = 10  # Reserved */
    SHT_DYNSYM = 11  # Dynamic linker symbol table */
    SHT_INIT_ARRAY = 14  # Array of constructors */
    SHT_FINI_ARRAY = 15  # Array of destructors */
    SHT_PREINIT_ARRAY = 16  # Array of pre-constructors */
    SHT_GROUP = 17  # Section group */
    SHT_SYMTAB_SHNDX = 18  # Extended section indices */
    SHT_NUM = 19  # Number of defined types. */
    SHT_LOOS = 0x60000000  # Start OS-specific. */
    SHT_GNU_ATTRIBUTES = 0x6FFFFFF5  # Object attributes. */
    SHT_GNU_HASH = 0x6FFFFFF6  # GNU-style hash table. */
    SHT_GNU_LIBLIST = 0x6FFFFFF7  # Prelink library list */
    SHT_CHECKSUM = 0x6FFFFFF8  # Checksum for DSO content. */
    SHT_LOSUNW = 0x6FFFFFFA  # Sun-specific low bound. */
    SHT_SUNW_move = 0x6FFFFFFA
    SHT_SUNW_COMDAT = 0x6FFFFFFB
    SHT_SUNW_syminfo = 0x6FFFFFFC
    SHT_GNU_verdef = 0x6FFFFFFD  # Version definition section. */
    SHT_GNU_verneed = 0x6FFFFFFE  # Version needs section. */
    SHT_GNU_versym = 0x6FFFFFFF  # Version symbol table. */
    SHT_HISUNW = 0x6FFFFFFF  # Sun-specific high bound. */
    SHT_HIOS = 0x6FFFFFFF  # End OS-specific type */
    SHT_LOPROC = 0x70000000  # Start of processor-specific */
    SHT_HIPROC = 0x7FFFFFFF  # End of processor-specific */
    SHT_LOUSER = 0x80000000  # Start of application-specific */
    SHT_HIUSER = 0x8FFFFFFF  # End of application-specific */


class SHF(IntFlag):
    SHF_WRITE = 1 << 0  # Writable */
    SHF_ALLOC = 1 << 1  # Occupies memory during execution */
    SHF_EXECINSTR = 1 << 2  # Executable */
    SHF_MERGE = 1 << 4  # Might be merged */
    SHF_STRINGS = 1 << 5  # Contains nul-terminated strings */
    SHF_INFO_LINK = 1 << 6  # `sh_info' contains SHT index */
    SHF_LINK_ORDER = 1 << 7  # Preserve order after combining */
    SHF_OS_NONCONFORMING = 1 << 8  # Non-standard OS specific handling
    SHF_GROUP = 1 << 9  # Section is member of a group. */
    SHF_TLS = 1 << 10  # Section hold thread-local data. */
    SHF_COMPRESSED = 1 << 11  # Section with compressed data. */
    SHF_MASKOS = 0x0FF00000  # OS-specific. */
    SHF_MASKPROC = 0xF0000000  # Processor-specific */
    SHF_GNU_RETAIN = 1 << 21  # Not to be GCed by linker. */
    SHF_ORDERED = 1 << 30  # Special ordering requirement
    SHF_EXCLUDE = 1 << 31  # Section is excluded unless


class R_386(IntFlag):
    R_386_NONE = 0  # No reloc */
    R_386_32 = 1  # Direct 32 bit */
    R_386_PC32 = 2  # PC relative 32 bit */
    R_386_GOT32 = 3  # 32 bit GOT entry */
    R_386_PLT32 = 4  # 32 bit PLT address */
    R_386_COPY = 5  # Copy symbol at runtime */
    R_386_GLOB_DAT = 6  # Create GOT entry */
    R_386_JMP_SLOT = 7  # Create PLT entry */
    R_386_RELATIVE = 8  # Adjust by program base */
    R_386_GOTOFF = 9  # 32 bit offset to GOT */
    R_386_GOTPC = 10  # 32 bit PC relative offset to GOT */
    R_386_32PLT = 11
    R_386_TLS_TPOFF = 14  # Offset in static TLS block */
    R_386_TLS_IE = 15  # Address of GOT entry for static TLS
    R_386_TLS_GOTIE = 16  # GOT entry for static TLS block
    R_386_TLS_LE = 17  # Offset relative to static TLS
    R_386_TLS_GD = 18  # Direct 32 bit for GNU version of
    R_386_TLS_LDM = 19  # Direct 32 bit for GNU version of
    R_386_16 = 20
    R_386_PC16 = 21
    R_386_8 = 22
    R_386_PC8 = 23
    R_386_TLS_GD_32 = 24  # Direct 32 bit for general dynamic
    R_386_TLS_GD_PUSH = 25  # Tag for pushl in GD TLS code */
    R_386_TLS_GD_CALL = 26  # Relocation for call to
    R_386_TLS_GD_POP = 27  # Tag for popl in GD TLS code */
    R_386_TLS_LDM_32 = 28  # Direct 32 bit for local dynamic
    R_386_TLS_LDM_PUSH = 29  # Tag for pushl in LDM TLS code */
    R_386_TLS_LDM_CALL = 30  # Relocation for call to
    R_386_TLS_LDM_POP = 31  # Tag for popl in LDM TLS code */
    R_386_TLS_LDO_32 = 32  # Offset relative to TLS block */
    R_386_TLS_IE_32 = 33  # GOT entry for negated static TLS
    R_386_TLS_LE_32 = 34  # Negated offset relative to static
    R_386_TLS_DTPMOD32 = 35  # ID of module containing symbol */
    R_386_TLS_DTPOFF32 = 36  # Offset in TLS block */
    R_386_TLS_TPOFF32 = 37  # Negated offset in static TLS block */
    R_386_SIZE32 = 38  # 32-bit symbol size */
    R_386_TLS_GOTDESC = 39  # GOT offset for TLS descriptor. */
    R_386_TLS_DESC_CALL = 40  # Marker of call through TLS
    R_386_TLS_DESC = 41  # TLS descriptor containing
    R_386_IRELATIVE = 42  # Adjust indirectly by program base */
    R_386_GOT32X = 43  # Load from 32 bit GOT entry,
    R_386_NUM = 44


class R_X86_64(IntEnum):
    R_X86_64_NONE = 0  # No reloc */
    R_X86_64_64 = 1  # Direct 64 bit */
    R_X86_64_PC32 = 2  # PC relative 32 bit signed */
    R_X86_64_GOT32 = 3  # 32 bit GOT entry */
    R_X86_64_PLT32 = 4  # 32 bit PLT address */
    R_X86_64_COPY = 5  # Copy symbol at runtime */
    R_X86_64_GLOB_DAT = 6  # Create GOT entry */
    R_X86_64_JUMP_SLOT = 7  # Create PLT entry */
    R_X86_64_RELATIVE = 8  # Adjust by program base */
    R_X86_64_GOTPCREL = 9  # 32 bit signed PC relative
    R_X86_64_32 = 10  # Direct 32 bit zero extended */
    R_X86_64_32S = 11  # Direct 32 bit sign extended */
    R_X86_64_16 = 12  # Direct 16 bit zero extended */
    R_X86_64_PC16 = 13  # 16 bit sign extended pc relative */
    R_X86_64_8 = 14  # Direct 8 bit sign extended */
    R_X86_64_PC8 = 15  # 8 bit sign extended pc relative */
    R_X86_64_DTPMOD64 = 16  # ID of module containing symbol */
    R_X86_64_DTPOFF64 = 17  # Offset in module's TLS block */
    R_X86_64_TPOFF64 = 18  # Offset in initial TLS block */
    R_X86_64_TLSGD = 19  # 32 bit signed PC relative offset
    R_X86_64_TLSLD = 20  # 32 bit signed PC relative offset
    R_X86_64_DTPOFF32 = 21  # Offset in TLS block */
    R_X86_64_GOTTPOFF = 22  # 32 bit signed PC relative offset
    R_X86_64_TPOFF32 = 23  # Offset in initial TLS block */
    R_X86_64_PC64 = 24  # PC relative 64 bit */
    R_X86_64_GOTOFF64 = 25  # 64 bit offset to GOT */
    R_X86_64_GOTPC32 = 26  # 32 bit signed pc relative
    R_X86_64_GOT64 = 27  # 64-bit GOT entry offset */
    R_X86_64_GOTPCREL64 = 28  # 64-bit PC relative offset
    R_X86_64_GOTPC64 = 29  # 64-bit PC relative offset to GOT */
    R_X86_64_GOTPLT64 = 30  # like GOT64, says PLT entry needed */
    R_X86_64_PLTOFF64 = 31  # 64-bit GOT relative offset
    R_X86_64_SIZE32 = 32  # Size of symbol plus 32-bit addend */
    R_X86_64_SIZE64 = 33  # Size of symbol plus 64-bit addend */
    R_X86_64_GOTPC32_TLSDESC = 34  # GOT offset for TLS descriptor. */
    R_X86_64_TLSDESC_CALL = 35  # Marker for call through TLS
    R_X86_64_TLSDESC = 36  # TLS descriptor. */
    R_X86_64_IRELATIVE = 37  # Adjust indirectly by program base */
    R_X86_64_RELATIVE64 = 38  # 64-bit adjust by program base */
    R_X86_64_GOTPCRELX = 41  # Load from 32 bit signed pc relative
    R_X86_64_REX_GOTPCRELX = 42  # Load from 32 bit signed pc relative
    R_X86_64_NUM = 43


class STT(IntEnum):
    STT_NOTYPE = 0  # Symbol type is unspecified */
    STT_OBJECT = 1  # Symbol is a data object */
    STT_FUNC = 2  # Symbol is a code object */
    STT_SECTION = 3  # Symbol associated with a section */
    STT_FILE = 4  # Symbol's name is file name */
    STT_COMMON = 5  # Symbol is a common data object */
    STT_TLS = 6  # Symbol is thread-local data object*/
    STT_NUM = 7  # Number of defined types. */
    STT_LOOS = 10  # Start of OS-specific */
    STT_GNU_IFUNC = 10  # Symbol is indirect code object */
    STT_HIOS = 12  # End of OS-specific */
    STT_LOPROC = 13  # Start of processor-specific */
    STT_HIPROC = 15  # End of processor-specific */


class STB(IntEnum):
    STB_LOCAL = 0  # Local symbol */
    STB_GLOBAL = 1  # Global symbol */
    STB_WEAK = 2  # Weak symbol */
    STB_NUM = 3  # Number of defined types. */
    STB_LOOS = 10  # Start of OS-specific */
    STB_GNU_UNIQUE = 10  # Unique symbol. */
    STB_HIOS = 12  # End of OS-specific */
    STB_LOPROC = 13  # Start of processor-specific */
    STB_HIPROC = 15  # End of processor-specific */


class STV(IntEnum):
    STV_DEFAULT = 0  # Default symbol visibility rules */
    STV_INTERNAL = 1  # Processor specific hidden class */
    STV_HIDDEN = 2  # Sym unavailable in other modules */
    STV_PROTECTED = 3  # Not preemptible, not exported */
