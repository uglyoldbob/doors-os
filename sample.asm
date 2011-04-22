[BITS 16]
jmp Main
FileName db 'DOORS   BIN'		;the name of the kernel file
Oops db 'DoorsBin missing', 13, 10, 0		;error message
SectorsLeft dd 0x0000			;number of sectors we havent already read in the kernel
PreCluster dw 0x0000			;stores the number of bytes before the clusters
Cluster dw 0x0000				;stores the cluster
FatLoc dd 0x00000000			;where the FAT is located
;FixMe dd 0x00000000			;I hope this fixes something
BytesSector dw 0x0000			;read from BPB
SectorsCluster db 0x00			;read from BPB
SectorsTrack dw 0x0000			;read from BPB
NumHeads dw 0x0000			;read from BPB
DriveNum db 0x00				;read from BPB
Type db 0x00				;this is used to get several types of memory (for int15 ax = E820)
Main:
	mov ax, 0x50
	mov ds, ax		;set the data segment to allow data access
	mov es, ax		;the extra segment is where the root directory and FAT table will be loaded
	mov ax, 0x07C0
	mov fs, ax		;the location of stuff for the BPB fields (required so we can read a file from disk)
				;should go after the last part of the kernel (so it should be calcualated at runtime)
			;the stack is already set up by our bootsector (and is known to be out of the way)
	mov ax, [FS:0x0B]
	mov [BytesSector], ax
	mov al, [FS:0x0D]
	mov [SectorsCluster], al
	mov ax, [FS:0x18]
	mov [SectorsTrack], ax
	mov ax, [FS:0x1A]
	mov [NumHeads], ax
	mov al, [FS:0x24]
	mov [DriveNum], al
;calculate location of root directory and load into memory
	xor cx, cx			;cx = 0
	xor dx, dx			;so we can multiply
	mov ax, 0020h		;size of a directory entry
	mul WORD [FS:0x11]	;multiply by maximum root entries
	div WORD [BytesSector]	;sectors used by root directory
	xchg ax, cx	
	mov al, [FS:0x10]		;number of FATs (BYTE)
	mul WORD [FS:0x16]	;AX * sectors per fat = DX:AX
	add ax, [FS:0x0E]	;add in bootsector sectors	(here was the mistake)
	mov [PreCluster], ax	;store that value
	add [PreCluster], cx	;add in the size of the root directory
	mov bx, 0x0400		;right above this transition program file
	call ReadSectors		;read the root directory
;search for the kernel
	mov cx, [FS:0x11]		;retrieve max root entries
	mov di, 0x0400		;locate the first entry
Looping:
	push cx		;save cx, becuase this is a nested loop
	mov cx, 0x000B	;eleven characters in a filename
	mov si, FileName	;the location of the name of the filename (not zero terminated!)
	push di
repe	cmpsb			;is it a match?
	pop di
	je FatRead		;it is a match
	pop cx		;we are out of the inner loop
	add di, 0x0020	;next directory entry
	loop Looping
	mov si, Oops
	call DisplayMessage
	jmp $			;freeze up so se know that we are stupid
FatRead:
	;it is now time to read the FAT, but we have to determine where to put it
	;size of fat = sizeof fat [16h]* numFATS[10h]
	mov dx, WORD[di + 0x001A]	;the first cluster of the kernel file
	mov WORD [Cluster], dx
	xor ax, ax
	mov al, BYTE [FS:0x10]		;the number of FATs
	mul WORD [FS:0x16]		;sectors per FAT
	mov cx, ax
	mov ax, WORD [FS:0x0E]		;reserved sectors (sectors before the first FAT)
						;corrected here too
	mov ebx, DWORD[di + 0x001C]   ;bx now contains the size of the kernel (bytes)
						;the location where we read the FAT to (0050:FatLoc)
	;we have to bytesPerSectorAlign the FAT address
	;divide by BytesPerSector, truncate that, add 1, mul by BytesPerSector
	;avail: ebx, edx, eax
	push ax
	mov eax, ebx			;for division (to align to bytes per sector)
	div WORD [BytesSector]		;quotient = eax
	add eax, 3				; += (BytesPerSector * 3)
	mov [SectorsLeft], eax		;Sectors we havent read from the kernel
	mul WORD [BytesSector]		;EDX:EAX
	mov [FatLoc], eax
	mov ebx, eax
	pop ax
	call ReadSectors			;read the sectors into memory
;we have located the bottom of the FAT table in RAM				
;read kernel into memory (090:0000, then "return" to it, transferring control to the kernel
	mov ax, 0x0090          ;destination of image CS
	mov es, ax			;forgot to do this
	mov bx, 00h
	push    bx
Load:
	mov ax, WORD [Cluster]		;cluster to read
	pop bx				;buffer to read to
	call ClusterLBA
	xor cx, cx
	mov cl, BYTE [SectorsCluster]	;sectors per cluster (number of sectors to read)
	call ReadSectors
	push bx
;calculate next cluster
	mov ax, WORD [Cluster]		;what is the current cluster?
	mov cx, ax
	mov dx, ax				;copy that number a couple times
	shr dx, 1h
	add cx, dx				;sum for (3/2)
	mov ebx, [FatLoc]			;location of FAT in RAM
	add bx, cx				;index into FAT (so we can read the next cluster number and do some stuff to it)
	mov dx, WORD [bx]			;read two bytes from FAT (12 bits requires two bytes to be read and 4 bits thrown away)
	test ax, 0001h			;check for type (even or odd cluster)
	jnz Odd
Even:
	and dx, 0000111111111111b	;we only want the lower twelve bits
	jmp Done
Odd:
	shr dx, 0004h			;take high twelve bits
Done:
	mov WORD [Cluster], dx		;store the new cluster value
	cmp dx, 0FF0h			;test for EOF
	jb Load				;keep reading if not done
;time to check memory sizes and some other stuff
;use newer calls first, and if they dont work, use the older calls
;check conventional memory
;create a stack (grows downward, starts at 0x9F800, ends with base 0, size 0
;entry is 8 bytes long aka 2 * eax ...
;mov eax, [length]
;mov [Address], eax
;sub [Address], 4
;mov eax, [base]
;mov [address], eax 
;sub [address], 4
;will ready eax to write an entry to ram
;mov eax, [somewhere]
;mov [base], eax
;add [somewhere], 4
;mov eax, [somewhere]
;mov [length], eax
;add [somewhere], 4
;this will retrieve one entry
;base, length
	mov eax, 0x9000
	mov fs, eax
	mov eax, 0xF7FC
	mov [Address], eax
	mov BYTE [Type], 1		;OS usable memory
	call extmem_int15_e820
	jc OldComputer
	;mov si, mem_msg2
	;call DisplayMessage
	;mov BYTE [Type], 2		;unusable memory
	;call extmem_int15_e820
	jnc ok
OldComputer:				;does not support the special calls
; before trying other BIOS calls, use INT 12h to get conventional memory size
	int 0x12
	push ax
	xor eax, eax
	pop ax
	;ax contains the number of kilobytes starting at 0	
	shl eax, 10
	;eax now contains length in bytes
	mov ebx, [Address]
	mov [fs:ebx], eax
	sub DWORD [Address], 4
	mov ebx, [Address]
	xor eax, eax
	mov [fs:ebx], eax
	sub DWORD [Address], 4
; try INT 15h AX=E801h
	call extmem_int15_e801
	jnc ok
; try INT 15h AH=88h
	call extmem_int15_88
	jnc ok
; uh-oh
	mov si,err_msg
	call DisplayMessage
ok:
	;dont forget to set the blank entry at the bottom
	mov ebx, [Address]
	xor eax, eax
	mov [fs:ebx], eax
	sub DWORD [Address], 4
	mov ebx, [Address]
	mov [fs:ebx], eax
;jump to the kernel now
	cli
	mov al, 0xFF	;disable IRQ's
	out 0x21, al
	xor ax, ax
	mov ds, ax              
	lgdt [gdt_desc + 0x0500]
	mov eax, cr0		;enable pmode
	or al, 1			
	mov cr0, eax
	jmp 0x08:0x0900
;mem_msg2 db 'Other Ranges:', 13, 10, 0
;mem_msg db 'Memory ranges:'
;crlf_msg db 13, 10, 0
;base_msg db 'base=0x', 0
;size_msg db ', size=0x', 0
err_msg db 'BIOS calls failed', 13, 10, 0

buffer_e820:
	times 20h db 0
buffer_e820_len	equ $ - buffer_e820

extmem_int15_e820:
	push es
	pushad
	push ds
	pop es
	mov di, buffer_e820
	xor ebx, ebx		; INT 15h AX=E820h continuation value
	mov edx, 534D4150h	; "SMAP"
	mov ecx, buffer_e820_len
	mov eax, 0000E820h
	int 15h
	jc extmem_e820_4
extmem_e820_1:
	cmp eax, 534D4150h	; "SMAP"
	stc
	jne extmem_e820_4
	push eax
	xor eax, eax
	mov al, [Type]
	cmp dword [es:di + 16], eax ; type [Type] memory (available to OS)
	pop eax
	jne extmem_e820_2
	push bx
	mov ebx, [Address]
	mov eax, [es:di + 8]	;length
	mov [fs:ebx], eax
	sub DWORD [Address], 4
	mov ebx, [Address]
	mov eax, [es:di]	;base
	mov [fs:ebx], eax
	sub DWORD [Address], 4
	pop bx
extmem_e820_2:
	or ebx,ebx
	je extmem_e820_3
	mov edx, 534D4150h	; "SMAP"
	mov ecx, buffer_e820_len
	mov eax, 0000E820h
	int 15h
	jnc extmem_e820_1
extmem_e820_3:
	clc
extmem_e820_4:
	popad
	pop es
	ret

extmem_int15_e801:
	pusha
	mov ax,0E801h
	xor dx,dx
	xor cx,cx
	int 15h
	jc extmem_e801_2
	mov si,ax
	or si,bx
	jne extmem_e801_1
	mov ax,cx
	mov bx,dx
extmem_e801_1:
	push bx
; convert from Kbytes in AX to bytes in CX:BX
	push ax
	xor eax, eax
	pop ax
	shl ax, 16
	mov ax, bx
	shl eax, 10
;eax contains bytes now
	mov ebx, [Address]
	mov [fs:ebx], eax
	sub DWORD [Address], 4
	mov ebx, [Address]
	mov eax, 0x100000
	mov [fs:ebx], eax
	sub DWORD [Address], 4
; convert stacked value from 64K-blocks to bytes in CX:BX
	pop cx
	mov ax, cx
	shl eax, 16
	mov ebx, [Address]
	mov [fs:ebx], eax
	sub DWORD [Address], 4
	mov ebx, [Address]
	mov eax, 0x1000000	;64M
	mov [fs:ebx], eax
	sub DWORD [Address], 4
extmem_e801_2:
	popa
	ret

extmem_int15_88:
	pusha
	mov ax,8855h
	int 15h
	cmp al,55h
	jne extmem_int15_1
	mov ax,88AAh
	int 15h
	cmp al,0AAh
	stc
	je extmem_int15_2
extmem_int15_1:
;ax is number of KB at 1MB
	push ax
	xor eax, eax
	pop ax
	shl eax, 10
;eax contains bytes at 1MB
	mov ebx, [Address]
	mov [fs:ebx], eax
	sub DWORD [Address], 4
	mov ebx, [Address]
	mov eax, 0x100000
	mov [fs:ebx], eax
	sub DWORD [Address], 4
extmem_int15_2:
	popa
	ret

DisplayMessage:
	push ax
	push bx
.Next
	lodsb                                       ; load next character
	or      al, al                              ; test for NUL character
	jz      .DONE
	mov     ah, 0x0E                            ; BIOS teletype
	mov     bh, 0x00                            ; display page 0
	mov     bl, 0x07                            ; text attribute
	int     0x10                                ; invoke BIOS
	jmp     .Next
.DONE:
	pop bx
	pop ax
	ret

gdt:                    ; Address for the GDT
gdt_null:               ; Null Segment
	dd 0
	dd 0
gdt_code:               ; Code segment, read/execute, nonconforming
	dw 0xFFFF
	dw 0x0000
	db 0
	db 10011010b	;non-system descriptor (bit 4)
	db 11001111b
	db 0
gdt_data:               ; Data segment, read/write
	dw 0xFFFF
	dw 0x0000
	db 0
	db 10010010b	;non system descriptor (bit 4)
	db 11001111b
	db 0
gdt_end:				; Used to calculate the size of the GDT
gdt_desc:				; The GDT descriptor
	dw gdt_end - gdt - 1	; Limit (size)
	dd gdt + 0x0500		; Address of the GDT

Cylinder db 0x00			;for the ReadSectors routine
Sector db 0x00			;for the ReadSectors routine
Head db 0x00			;for the ReadSectors routine

ReadSectors:			;this reads multiple sectors in a row, one sector at a time
					;ax = begin; es:bx = mem location; cx = how many sectors
					;I choose this method to make reading sectors easier
	push cx			;save number of sectors read (so we can place the kernel file appropiately)
.Main
	mov di, 0x0005		;five tries if there are errors
.SectorLoop
	push ax
	push bx
	push cx
	xor dx, dx			;clear dx for division
	div WORD [SectorsTrack]	;divide ax (beginning of root in LBA) by bytes per sector
	inc dl			;adjust for sector 0
	mov [Sector], dl		;store that value
	xor dx, dx			;prepare to divide
	div WORD [NumHeads]	;divide by the number of heads
	mov [Head], dl		;store the value
	mov [Cylinder], al	;store the value
	mov ah, 02h			;we are reading sectors
	mov al, 01h			;read one sector
	mov ch, BYTE [Cylinder]	;restore the track value
	mov cl, BYTE [Sector]	;restore the sector value
	mov dh, BYTE [Head]	;restore the value
	mov dl, BYTE [DriveNum]	;retrieve the drive number as recognized by the BIOS int13 routine
	int 13h			;read one sector
	jnc .Success
	xor ax, ax			;reset disk
	int 13h			;call BIOS
	dec di			;decrement error counter
	pop cx
	pop bx
	pop ax
	jnz .SectorLoop		;read again if there was an error
;	int 18h
.Success
	pop cx
	pop bx
	pop ax
	add bx, [BytesSector]	;add in bytes per sector for the next buffer zone
	inc ax			;next sector to read
	loop .Main
	pop cx			;restore number of sectors read
	ret

ClusterLBA: 		;converts a FAT cluster number to LBA
	sub ax, 0x0002	;cluster number starts at 2
	xor cx, cx		;cx = 0
	mov cl, BYTE [SectorsCluster];sectors per cluster
	mul cx
	add ax, WORD [PreCluster]	;add in sectors before the first cluster
	ret			;go back to where yee came from :)

Address dd 0

times 1024-($-$$) db '?'	;this is the size of the future protected mode stack, if we go any larger, we will go
				;into kernel memory space, and that is VERY bad (1024 = 0x0400)
				;even if the file is larger on the disk (becuase of cluster / sector sizes), it wont be a problem
