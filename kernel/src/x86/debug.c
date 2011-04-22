#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif 

#include "spinlock.h"

#include "disasm/types.h"
#include "disasm/extern.h"
#include "disasm/decode.h"
#include "disasm/input.h"
#include "disasm/itab.h"
#include "disasm/syn.h"

#include "debug.h"

#include <stdio.h>

//exact breakpoints are enabled by default
//detection of access to debug registers


//debug exception
//breakpoint exception
//resume and trap flags (eflags)
//trap flags (tss)

EXTERNC int getdisasmb(struct ud* disasm)
{
	printf("Here is the byte grabber for the disassembler\n");
	return 0x90;
}
