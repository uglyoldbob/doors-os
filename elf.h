#include "file.h"

//base address - lowest virtual address associated with the memory image of the object file
	//usually used for relocation
	//calculate using: memory load address, maximum page size, lowest virtual address
//segments must use the same relations to each other


#define ET_NONE 		0		//no file type
#define ET_REL			1		//relocatable file
#define ET_EXEC			2		//executable file
#define ET_DYN			3		//shared object file
#define ET_CORE			4		//core file
#define ET_LOPROC		0xFF00	//processor-specific
#define ET_HIPROC		0xFFFF	//processor-specific

#define EM_NONE			0		//no machine
#define EM_M32			1		//AT&T WE 32100
#define EM_SPARC		2		//SPARC
#define EM_386			3		//intel 80386
#define EM_68K			4		//motorola 68000
#define EM_88K			5		//motorola 88000
#define EM_860			7		//intel 80860
#define EM_MIPS			8		//mips RS3000

#define EV_NONE			0		//invalid version
#define EV_CURRENT		1		//current version

#define EI_MAG0			0		//file identification
#define EI_MAG1			1		//file identification
#define EI_MAG2			2		//file identification
#define EI_MAG3			3		//file identification
#define EI_CLASS		4		//file class
#define EI_DATA			5		//data encoding
#define EI_VERSION		6		//file version
#define EI_PAD			7		//start of padding bytes
#define EI_NIDENT		16		//size of e_ident[] (not present or invalid in the object file I tested with)

#define	ELFMAG0			0x7F
#define ELFMAG1			'E'
#define ELFMAG2			'L'
#define ELFMAG3			'F'

#define ELFCLASSNONE	0		//invalid class
#define ELFCLASS32		1		//32-bit objects
#define ELFCLASS64		2		//64-bit-objects

#define ELFDATANONE		0		//invalid data encoding
#define ELFDATA2LSB		1		//lsb encoding
#define ELFDATA2MSB		2		//msb encoding

#define SHN_UNDEF		0
#define SHN_LORESERVE	0xFF00	//lower bound of the reserved indexes
#define SHN_LOPROC		0xFF00	//processor specific
#define SHN_HIPROC		0xFF1F	//processor specific
#define SHN_ABS			0xFFF1	//absolute values, unaffected by relocation
#define SHN_COMMON		0xFFF2	//common symbols (fortran symbols, unallocated C external variables)
#define SHN_HIRESERVE	0xFFFF	//the upper bound of reserved indexes

#define SHT_NULL		0			//inactive section
#define SHT_PROGBITS	1			//information is defined by the program
#define SHT_SYMTAB		2			//symbol table
#define SHT_STRTAB		3			//string table
#define SHT_RELA		4			//relocation entries with explicit addends
#define SHT_HASH		5			//symbol hash table
#define SHT_DYNAMIC		6			//dynamic linking information
#define SHT_NOTE		7			//
#define SHT_NOBITS		8			//takes no space in the file, but takes space in memory
#define SHT_REL			9			//relocation entries without explicit addends
#define SHT_SHLIB		10			//reserved, but has unspecified semantics, this section does not conform to the ABI
#define SHT_DYNSYM		11			//symbol table
#define SHT_LOPROC		0x70000000	//processor specific
#define SHT_HIPROC		0x7FFFFFFF	//processor specific
#define SHT_LOUSER		0x80000000	//lower bound of indexes reserved for application programs
#define SHT_HIUSER		0xFFFFFFFF	//upper bound of indexes reserved for application programs

#define SHF_WRITE		0x1			//writable
#define SHF_ALLOC		0x2			//occupies memory
#define SHF_EXECINSTR	0x4			//code
#define SHF_MASKPROC	0xF0000000	//processor specific

#define STN_UNDEF		0

#define STB_LOCAL		0			//local symbol
#define STB_GLOBAL		1			//global symbol
#define STB_WEAK		2			//global symbols of lower precedence
#define STB_LOPROC		13			//processor specific
#define STB_HIPROC		15			//processor specific
//multiple global symbols of the same name are not allowed
	//if a global symbol exists and then a weak symbol comes along, an error is not generated
		//global symbols are honored and weak ones are ignored
		//unresolved weak symbols have a value of 0

#define STT_NOTYPE		0			//type not specified
#define STT_OBJECT		1			//associated with a data object
#define STT_FUNC		2			//associated with a function
#define STT_SECTION		3			//associated with a section
#define STT_FILE		4			//object file
#define STT_LOPROC		13			//processor specific
#define STT_HIPROC		15			//processor specific


//A = addend
//B = base address at which a shared object has been loaded for execution
//G = offset into the global offset table, which is where the address for the relocation will be
//GOT = address of the global offset table
//L = place of the procedure linkage table entry for the symbol
//P = place of the storage unit being relocated
//S = value of the symbol whose index resides in the relocation entry
#define R_386_NONE		0			//
#define R_386_32		1			//S + A
#define R_386_PC32		2			//S + A - P
#define R_386_GOT32		3			//G + A - P
	//compute the distance from the base of the GOT to the GOT entry, also build a GOT
#define R_386_PLT32		4			//L + A - P
	//the address of the PLT entry, also build a PLT
#define R_386_COPY		5			//
	//dynamic linking, offset member refers to a location in a writable segment
#define R_386_GLOB_DAT	6			//S
	//set a GOT entry to the address of the specified symbol
#define R_386_JMP_SLOT	7			//S
	//dynamic linking, location is the location of a PLT entry 
#define R_386_RELATIVE	8			//B + A
	//dynamic linking, a location within a shared object that contains a value representing a relative address
#define R_386_GOTOFF	9			//S + A - GOT
	//difference between the symbol's value and the address of the GOT, also build a GOT
#define R_386_GOTPC		10			//GOT + A - P
	//like R_386_PC32, except it uses the GOT, also build the GOT

#define PT_NULL			0			//nothing
#define PT_LOAD			1			//loadable segment, 
#define PT_DYNAMIC		2			//dynamic linking information
#define PT_INTERP		3			//null-terminated path to a interpreter
#define PT_NOTE			4			//notes
#define PT_SHLIB		5			//unspecified, if present it doesnt conform to the ABI
#define PT_PHDR			6			//location and size of the program header table, both in file and memory
#define PT_LOPROC		0x70000000	//processor specific
#define PT_HIPROC		0x7FFFFFFF	//processor specific

typedef void * Elf32_Addr;
typedef unsigned short 	Elf32_Half;
typedef unsigned long	Elf32_Off;
typedef signed long		Elf32_Sword;
typedef unsigned long	Elf32_Word;

#define ELF32_ST_BIND(i)   ((i)>>4)
#define ELF32_ST_TYPE(i)   ((i)&0xf)
#define ELF32_ST_INFO(b,t) (((b)<<4)+((t)&0xf))

struct elf32_rel
{
	Elf32_Addr	  r_offset;	//where to apply relocation stuff
	Elf32_Word	  r_info;	//symbol table index, type of relocation, 
};

struct elf32_rela
{
	Elf32_Addr	  r_offset;	//where to apply relocation stuff
	Elf32_Word	  r_info;	//symbol table index, type of relocation, 
	Elf32_Sword	  r_addend;	//addend used to compute the value to be stored into the relocatable field
};


struct elf_symbol_table_entry
{
	Elf32_Word    st_name;
	Elf32_Addr    st_value;
	Elf32_Word    st_size;
	unsigned char st_info;
	unsigned char st_other;
	Elf32_Half    st_shndx;
};

struct elf_program_header
{
	Elf32_Word p_type;		//the kind of segment
	Elf32_Off  p_offset;	//offset from the beginning of the file
	Elf32_Addr p_vaddr;		//virtual address for the first byte of the segment
	Elf32_Addr p_paddr;		//the segment's physical address
	Elf32_Word p_filesz;	//number of bytes used by the segment in the file
	Elf32_Word p_memsz;		//number of bytes used in memory
	Elf32_Word p_flags;		//flags
	Elf32_Word p_align;		//alignment requirements
};

struct elf_section_header
{
	Elf32_Word 			sh_name;
	Elf32_Word 			sh_type;
	Elf32_Word 			sh_flags;
	Elf32_Addr 			sh_addr;
	Elf32_Off  			sh_offset;
	Elf32_Word 			sh_size;
	Elf32_Word 			sh_link;
	Elf32_Word 			sh_info;
	Elf32_Word 			sh_addralign;
	Elf32_Word 			sh_entsize;
};

struct elf_header
{
	unsigned char 	   *e_ident;
	Elf32_Half 			e_type;
	Elf32_Half 			e_machine;
	Elf32_Word 			e_version;
	Elf32_Addr 			e_entry;
	Elf32_Off  			e_phoff;
	Elf32_Off  			e_shoff;
	Elf32_Word 			e_flags;
	Elf32_Half 			e_ehsize;
	Elf32_Half 			e_phentsize;
	Elf32_Half 			e_phnum;
	Elf32_Half 			e_shentsize;
	Elf32_Half 			e_shnum;
	Elf32_Half 			e_shstrndx;
	elf_section_header *e_sheaders;
};

int load_module(char *filename, filesystem *fs);



