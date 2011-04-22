[BITS 16]
jmp Main
FileName db 'DOORS   BIN'		;the name of the kernel file
Oops db 'Oops...', 13, 10, 0		;error message
SectorsLeft dd 0x0000			;number of sectors we havent already read in the kernel
PreCluster dw 0x0000			;stores the number of bytes before the clusters
Cluster dw 0x0000				;stores the cluster
FatLoc dd 0x00000000			;where the FAT is located
FixMe dd 0x00000000			;I hope this fixes something
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
	mov BYTE [Type], 1		;OS usable memory
	call extmem_int15_e820
	jc OldComputer
	mov si, mem_msg2
	call DisplayMessage
	mov BYTE [Type], 2		;unusable memory
	call extmem_int15_e820
	jnc ok
OldComputer:				;does not support the special calls
; before trying other BIOS calls, use INT 12h to get conventional memory size
	int 12h
; convert from K in AX to bytes in CX:BX		;i think this is where i figure out how the formula works
	xor ch, ch	
	xor ch,ch
	mov cl,ah
	mov bh,al
	xor bl,bl
	shl bx,1
	rcl cx,1
	shl bx,1
	rcl cx,1
; set range base (in DX:AX) to 0 and display it
	xor dx,dx
	xor ax,ax
	call display_range
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

;jump to the kernel now
	cli
	xor ax, ax
	mov ds, ax              
	lgdt [gdt_desc + 0x0500]
	mov eax, cr0		;enable pmode
	or al, 1			
	mov cr0, eax
	jmp 0x08:0x0900
mem_msg2 db 'Other Ranges:', 13, 10, 0
mem_msg db 'Memory ranges:'
crlf_msg db 13, 10, 0
base_msg db 'base=0x', 0
size_msg db ', size=0x', 0
err_msg db 'BIOS calls to determine memory size has failed', 13, 10, 0
;ROUTINES
display_range:
	pusha
; if size==0, do nothing
	mov si, cx
	or si, bx
	je display_range_1
	mov si, base_msg
	call DisplayMessage
	push bx
	mov bx, 16
	call wrnum
	mov si, size_msg
	call DisplayMessage
	pop ax
	mov dx, cx
	call wrnum
	mov si, crlf_msg
	call DisplayMessage
display_range_1:
	popa
	ret


	times 40 db 0
num_buf:
	db 0

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
	mov ax, [es:di + 0] ; base
	mov dx, [es:di + 2]
	mov bx, [es:di + 8] ; size
	mov cx, [es:di + 10]
	call display_range
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
	xor ch,ch
	mov cl,ah
	mov bh,al
	xor bl,bl
	shl bx,1
	rcl cx,1
	shl bx,1
	rcl cx,1
; set range base (in DX:AX) to 1 meg and display it
	mov dx,10h
	xor ax,ax
	call display_range
; convert stacked value from 64K-blocks to bytes in CX:BX
	pop cx
	xor bx,bx
; set range base (in DX:AX) to 16 meg and display it
	mov dx,100h
	xor ax,ax
	call display_range
extmem_e801_2:
	popa
	ret

wrnum:
	pusha
	mov si,num_buf
wrnum1:
	push ax
	mov ax,dx
	xor dx,dx
	div bx
	mov cx,ax
	pop ax
	div bx
	xchg dx,cx
	add cl,'0'
	cmp cl,'9'
	jbe wrnum2
	add cl,('A'-('9'+1))
wrnum2:
	dec si
	mov [si],cl
	mov cx,ax
	or cx,dx
	jne wrnum1
	call DisplayMessage
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
	xor ch,ch
	mov cl,ah
	mov bh,al
	xor bl,bl
	shl bx,1
	rcl cx,1
	shl bx,1
	rcl cx,1
	mov dx,10h
	xor ax,ax
	call display_range
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

times 1024-($-$$) db '?'	;this is the size of the future protected mode stack, if we go any larger, we will go
				;into kernel memory space, and that is VERY bad (1024 = 0x0400)
				;even if the file is larger on the disk (becuase of cluster / sector sizes), it wont be a problem
