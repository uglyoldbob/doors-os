//header
#include "file.h"
#include "video.h"
#include "memory.h"
#include "elf.h"
#include "entrance.h"

/*
	typedef struct 
	{
        	Elf32_Word sh_name;
				//index into the section header string table for the name of the section
        	Elf32_Word sh_type;
        	Elf32_Word sh_flags;
        	Elf32_Addr sh_addr;
				//address for the first byte of the section (or 0)
        	Elf32_Off  sh_offset;
				//location in the file
        	Elf32_Word sh_size;
				//section size
        	Elf32_Word sh_link;
        	Elf32_Word sh_info;
        	Elf32_Word sh_addralign;
        	Elf32_Word sh_entsize;
	}	Elf32_Shdr;

*/

//special sections

//string table

//symbol table

//relocation


//modules will be relocatable

int load_module(char *filename, filesystem *fs)
{
	display("\nLoading module: ");
	display(filename);
	display("\n");

	elf_header load;
	krnl_FILE *file_test;
	file_test = open(filename, 0, fs);

	unsigned char *buffer;
	buffer = (unsigned char*)kmalloc(sizeof(unsigned char) * 17);
	unsigned long check;

	for (int a = 0; a < 16; a++)
		buffer[a] = get_b(file_test, fs);
	check = buffer[0]<<24;
	check += buffer[1]<<16;
	check += buffer[2]<<8;
	check += buffer[3];
	if (check != 0x7F454C46)
		//check for the magic signature (0x7F, 'E', 'L', 'F')
		return -1;
	switch(buffer[EI_CLASS])
	{
		case ELFCLASSNONE:
		{
			display("Invalid object\n");
			return -1;	break;
		}
		case ELFCLASS32:
		{
			display("32-bit object\n");
			break;
		}
		case ELFCLASS64:
		{
			display("64-bit object?\n");
			break;
		}
		default:
		{
			display("Unknown object type\n");
			break;
		}
	}
	switch(buffer[EI_DATA])
	{
		case ELFDATANONE:
		{
			display("Invalid data encoding\n");
			return -1;	break;
		}
		case ELFDATA2LSB:
		{
			display("LSB data encoding\n");
			break;
		}
		case ELFDATA2MSB:
		{
			display("MSB data encoding\n");
			break;
		}
		default:
		{
			display("Unknown data encoding\n");
			return -1;	break;
		}
	}
	if (buffer[EI_VERSION] == EV_NONE)
	{
		display("Invalid version\n");
		return -1;
	}
	else
	{
		display("Version: ");
		PrintNumber(buffer[EI_VERSION]);
		display("\n");
	}
	load.e_ident = buffer;
	load.e_type = get_w(file_test, fs);
	switch(load.e_type)
	{
		case ET_NONE:
			display("No file type\n");
			return -1;	break;
		case ET_REL:
			display("Relocatable file\n");
			return -1;	break;
		case ET_EXEC:
			display("Executable file\n");
			return -1;	break;
		case ET_DYN:
			display("Shared object file\n");
			break;
		case ET_CORE:
			display("Core file\n");
			return -1;	break;
		default:
			display("Other type of file\n");
			return -1;	break;
	}
	load.e_machine = get_w(file_test, fs);
	switch(load.e_type)
	{
		case EM_NONE:
			display("No machine type\n");
			return -1; break;
		case EM_M32:
			display("AT&T WE 32100\n");
			break;
		case EM_SPARC:
			display("SPARC\n");
			break;
		case EM_386:
			display("Intel 30386\n");
			break;
		case EM_68K:
			display("Motorola 68000\n");
			break;
		case EM_88K:
			display("Motorola 88000\n");
			break;
		case EM_860:
			display("Intel 80860\n");
			break;
		case EM_MIPS:
			display("MIPS RS3000\n");
			break;
		default:
			display("Unknown architecture\n");
			break;
	}
	load.e_version = get_dw(file_test, fs);
	display("Version: ");
	PrintNumber(load.e_version);
	display("\n");
	load.e_entry = (void*)get_dw(file_test, fs);
	display("Entry point: ");
	PrintNumber((unsigned long)load.e_entry);
	display("\n");
	load.e_phoff = get_dw(file_test, fs);
	display("Program header table offset: ");
	PrintNumber((unsigned long)load.e_phoff);
	display("\n");
	load.e_shoff = get_dw(file_test, fs);
	display("Section header table offset: ");
	PrintNumber((unsigned long)load.e_shoff);
	display("\n");
	load.e_flags = get_dw(file_test, fs);
	display("Flags: ");
	PrintNumber(load.e_flags);
	display("\n");
	load.e_ehsize = get_w(file_test, fs);
	display("ELF header size: ");
	PrintNumber(load.e_ehsize);
	display("\n");
	load.e_phentsize = get_w(file_test, fs);
	display("Size of one entry in the program header table: ");
	PrintNumber(load.e_phentsize);
	display("\n");
	load.e_phnum = get_w(file_test, fs);
	display("Number of entries in the program header table: ");
	PrintNumber(load.e_phnum);
	display("\n");
	load.e_shentsize = get_w(file_test, fs);
	display("Size of one entry in the section header table: ");
	PrintNumber(load.e_shentsize);
	display("\n");
	load.e_shnum = get_w(file_test, fs);
	display("Number of entries in the section header table: ");
	PrintNumber(load.e_shnum);
	display("\n");
	load.e_shstrndx = get_w(file_test, fs);
	display("Section header table index for the section name string table: ");
	PrintNumber(load.e_shstrndx);
	display("\n");
	
	//load section table
	if ( (load.e_shoff != 0) && (load.e_shnum != 0) )
	{	//make sure that the section header exists and that there are entries in it
		seek(file_test, load.e_shoff, fs);
		load.e_sheaders = (elf_section_header *)kmalloc(sizeof(elf_section_header) * load.e_shnum);
		for (unsigned int entry_number = 0; entry_number < load.e_shnum; entry_number++)
		{
			display("Section header: ");
			PrintNumber(entry_number);
			display("\n");
			load.e_sheaders[entry_number].sh_name = get_dw(file_test, fs);
			load.e_sheaders[entry_number].sh_type = get_dw(file_test, fs);
			load.e_sheaders[entry_number].sh_flags = get_dw(file_test, fs);
			load.e_sheaders[entry_number].sh_addr = (void*)get_dw(file_test, fs);
			load.e_sheaders[entry_number].sh_offset = get_dw(file_test, fs);
			load.e_sheaders[entry_number].sh_size = get_dw(file_test, fs);
			load.e_sheaders[entry_number].sh_link = get_dw(file_test, fs);
			load.e_sheaders[entry_number].sh_info = get_dw(file_test, fs);
			load.e_sheaders[entry_number].sh_addralign = get_dw(file_test, fs);
			load.e_sheaders[entry_number].sh_entsize = get_dw(file_test, fs);
			display("\tName:");
			PrintNumber(load.e_sheaders[entry_number].sh_name);
			display("\n\tType:");
			PrintNumber(load.e_sheaders[entry_number].sh_type);
			display("\n\tFlags:");
			PrintNumber(load.e_sheaders[entry_number].sh_flags);
			display("\n\tAddress:");
			PrintNumber((unsigned long)load.e_sheaders[entry_number].sh_addr);
			display("\n\tFile Offset:");
			PrintNumber(load.e_sheaders[entry_number].sh_offset);
			display("\n\tSize in the file:");
			PrintNumber(load.e_sheaders[entry_number].sh_size);
			display("\n\tSHT index link:");
			PrintNumber(load.e_sheaders[entry_number].sh_link);
			display("\n\tExtra info:");
			PrintNumber(load.e_sheaders[entry_number].sh_info);
			display("\n\tAlignment:");
			PrintNumber(load.e_sheaders[entry_number].sh_addralign);
			display("\n\tEntry size:");
			PrintNumber(load.e_sheaders[entry_number].sh_entsize);
			display("\n");
			Delay(10000);
		}
	}
}
