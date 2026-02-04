; z80dasm 1.2.0
; command line: z80dasm -a -l -o trom34.asm roms/trom34.rom

	org 00100h

	jp 00070h		;0100
	jp 00357h		;0103
	jp 00662h		;0106
	jp l0494h		;0109
	jp l112ah		;010c
	jp 01066h		;010f
	jp l1166h		;0112
	jp l113ah		;0115
	jp l116bh		;0118
	jp 0130ah		;011b
	jp 0131fh		;011e
	jp 01137h		;0121
	jp l0f23h		;0124
	jp 00f49h		;0127
	jp 004f9h		;012a
	jp l0556h		;012d
	jp 005a8h		;0130
	jp l05b2h		;0133
	jp l05bdh		;0136
	jp l05c5h		;0139
	jp l05e7h		;013c
	jp l05eeh		;013f
	jp l05b7h		;0142
	jp l0696h		;0145
	jp 01635h		;0148
	jp l05dbh		;014b
	jp l05dfh		;014e
	jp l05d2h		;0151
	jp l05ceh		;0154
	jp l02f2h		;0157
	jp 00e4fh		;015a
	jp 00e4bh		;015d
	jp 01642h		;0160
	jp l102dh		;0163
	ret			;0166
	jp l03a5h		;0167
	jp l039ch		;016a
	jp 003b4h		;016d
	di			;0170
	ld sp,0ff96h		;0171
	ld hl,000f0h		;0174
l0177h:
	ld b,000h		;0177
l0179h:
	djnz l0179h		;0179
	dec hl			;017b
	ld a,h			;017c
l017dh:
	or l			;017d
	jr nz,l0177h		;017e
	call 00662h		;0180
	call l048bh+1		;0183
	call sub_15ffh		;0186
	call 0034bh		;0189
	dec de			;018c
	dec a			;018d
	ld hl,0413fh		;018e
	ld b,h			;0191
	ld d,(hl)		;0192
	ld b,l			;0193
	ld c,(hl)		;0194
	ld d,h			;0195
	jr nz,l01b8h		;0196
	ld d,b			;0198
	ld d,d			;0199
	ld c,a			;019a
	ld b,h			;019b
	ld d,l			;019c
	ld b,e			;019d
	ld d,h			;019e
	ld d,e			;019f
	dec de			;01a0
	dec a			;01a1
	inc hl			;01a2
	ld a,027h		;01a3
	jr c,$+54		;01a5
	jr nz,$+86		;01a7
	ld d,l			;01a9
	ld d,d			;01aa
	ld b,d			;01ab
	ld c,a			;01ac
	jr nz,l01cfh		;01ad
	ld d,d			;01af
	ld c,a			;01b0
	ld c,l			;01b1
	jr nz,l022ah		;01b2
	inc sp			;01b4
	ld l,034h		;01b5
	dec de			;01b7
l01b8h:
	dec a			;01b8
	dec h			;01b9
	ld a,(06328h)		;01ba
l01bdh:
	add hl,hl		;01bd
	jr nz,$+51		;01be
	add hl,sp		;01c0
	jr c,l01f8h		;01c1
	inc l			;01c3
	jr nz,l0216h		;01c4
	ld l,h			;01c6
	ld (hl),l		;01c7
	ld hl,(06550h)		;01c8
	ld (hl),d		;01cb
	ld h,(hl)		;01cc
	ld h,l			;01cd
	ld h,e			;01ce
l01cfh:
	ld (hl),h		;01cf
	jr nz,l0225h		;01d0
	ld a,c			;01d2
	ld (hl),e		;01d3
	ld (hl),h		;01d4
	ld h,l			;01d5
	ld l,l			;01d6
	ld (hl),e		;01d7
	dec de			;01d8
	dec a			;01d9
	jr z,l01fch		;01da
	nop			;01dc
	xor a			;01dd
	ld (0e7feh),a		;01de
	ld (0fe78h),a		;01e1
	call sub_1189h		;01e4
	xor a			;01e7
	out (086h),a		;01e8
	call 00ee7h		;01ea
	ld hl,0fe67h		;01ed
	ld bc,00807h		;01f0
	ld (hl),000h		;01f3
	ld a,(0fe65h)		;01f5
l01f8h:
	cp 003h			;01f8
	jr z,l01ffh		;01fa
l01fch:
	ld bc,00403h		;01fc
l01ffh:
	bit 0,c			;01ff
	call sub_0f0fh		;0201
	ld a,c			;0204
	dec c			;0205
	srl a			;0206
	push bc			;0208
	push hl			;0209
	call sub_10e7h		;020a
	pop hl			;020d
	pop bc			;020e
	call 00fa7h		;020f
	rla			;0212
	ccf			;0213
	rl (hl)			;0214
l0216h:
	djnz l01ffh		;0216
	bit 0,(hl)		;0218
	ld a,0ffh		;021a
	jr z,l022ah		;021c
	ld (hl),005h		;021e
	ld a,(0fe65h)		;0220
	cp 003h			;0223
l0225h:
	jr z,l0229h		;0225
	res 2,(hl)		;0227
l0229h:
	xor a			;0229
l022ah:
	set 0,(hl)		;022a
	inc hl			;022c
	ld (hl),a		;022d
	dec hl			;022e
	ld a,(hl)		;022f
	ld hl,0fe69h		;0230
	ld b,004h		;0233
l0235h:
	rrca			;0235
	rrca			;0236
	ld (hl),006h		;0237
	jr nc,l023dh		;0239
	ld (hl),003h		;023b
l023dh:
	inc hl			;023d
	ld (hl),032h		;023e
	inc hl			;0240
	ld (hl),019h		;0241
	inc hl			;0243
	djnz l0235h		;0244
	ld hl,0fda7h		;0246
	ld (0fff6h),hl		;0249
	xor a			;024c
	ld (hl),a		;024d
	inc a			;024e
	call 018bbh		;024f
	jr c,l027ch		;0252
	ld (0e7fdh),a		;0254
	ld hl,01784h		;0257
	ld de,00006h		;025a
	inc a			;025d
l025eh:
	add hl,de		;025e
	dec a			;025f
	jr nz,l025eh		;0260
	ld a,0ffh		;0262
	ld (0e7feh),a		;0264
	inc a			;0267
	ex af,af'		;0268
	ld a,011h		;0269
	ex af,af'		;026b
	ld e,(hl)		;026c
	inc hl			;026d
	ld d,(hl)		;026e
	ld bc,l0300h		;026f
l0272h:
	srl d			;0272
	rr e			;0274
	djnz l0272h		;0276
	inc de			;0278
	call l102dh		;0279
l027ch:
	ld a,(0fe67h)		;027c
	ld c,a			;027f
	ld b,004h		;0280
l0282h:
	xor a			;0282
	rlc c			;0283
	jr nc,l0289h		;0285
	set 7,a			;0287
l0289h:
	rlc c			;0289
	jr nc,l02a9h		;028b
	ex af,af'		;028d
	ld a,b			;028e
	dec a			;028f
	add a,a			;0290
	add a,a			;0291
	add a,a			;0292
	add a,a			;0293
	set 6,a			;0294
	bit 1,c			;0296
	push bc			;0298
	ld bc,00040h		;0299
	ld de,00019h		;029c
	jr z,l02a4h		;029f
	ld de,00032h		;02a1
l02a4h:
	call l102dh		;02a4
	pop bc			;02a7
	ex af,af'		;02a8
l02a9h:
	djnz l0282h		;02a9
	ld a,(0fe66h)		;02ab
	bit 0,a			;02ae
	jr z,l02d6h		;02b0
	bit 7,a			;02b2
	jr nz,l02d3h		;02b4
	ld bc,00009h		;02b6
	ex af,af'		;02b9
	ld a,c			;02ba
	set 7,a			;02bb
	ex af,af'		;02bd
	ld c,000h		;02be
	ld a,020h		;02c0
	ld de,0008dh		;02c2
	push af			;02c5
	call l102dh		;02c6
	pop af			;02c9
	ex af,af'		;02ca
	res 7,a			;02cb
	ex af,af'		;02cd
	call l102dh		;02ce
	jr l02d6h		;02d1
l02d3h:
	call 003bbh		;02d3
l02d6h:
	ld hl,(0fff6h)		;02d6
	dec hl			;02d9
	ld (hl),000h		;02da
	call sub_105eh		;02dc
	ld (0fff4h),hl		;02df
	ld a,(0e7feh)		;02e2
	or a			;02e5
	jp z,0021fh		;02e6
	call sub_105eh		;02e9
	ld (0fff4h),hl		;02ec
	inc hl			;02ef
	push hl			;02f0
	xor a			;02f1
l02f2h:
	ld b,012h		;02f2
l02f4h:
	ld (hl),a		;02f4
	inc hl			;02f5
	djnz l02f4h		;02f6
	pop hl			;02f8
	push hl			;02f9
	inc hl			;02fa
	inc hl			;02fb
	inc hl			;02fc
	ld (hl),040h		;02fd
	inc hl			;02ff
l0300h:
	inc hl			;0300
	ex de,hl		;0301
l0302h:
	ld a,(0e7fdh)		;0302
sub_0305h:
	inc a			;0305
	ld hl,01781h		;0306
	ld bc,00006h		;0309
	push bc			;030c
l030dh:
	add hl,bc		;030d
	dec a			;030e
	jr nz,l030dh		;030f
	pop bc			;0311
	ldir			;0312
	ex de,hl		;0314
	inc hl			;0315
	ld (hl),0c0h		;0316
	inc hl			;0318
	inc hl			;0319
	inc hl			;031a
	inc hl			;031b
	ld (hl),001h		;031c
	pop hl			;031e
	ld a,(0fe66h)		;031f
	rlca			;0322
	call c,00417h		;0323
	ld hl,(0fff6h)		;0326
	ld b,(hl)		;0329
	ld de,0000fh		;032a
	add hl,de		;032d
	ld de,(0fff4h)		;032e
l0332h:
	call sub_035fh		;0332
	inc hl			;0335
	inc hl			;0336
	call sub_035fh		;0337
	push de			;033a
	ld de,00010h		;033b
	add hl,de		;033e
	pop de			;033f
	djnz l0332h		;0340
	ex de,hl		;0342
	ld a,(0fe66h)		;0343
	rlca			;0346
	call c,sub_0464h	;0347
	ld de,0fc00h		;034a
	add hl,de		;034d
	ld (0fff2h),hl		;034e
	xor a			;0351
	call sub_10e7h		;0352
	call sub_0f06h		;0355
	call sub_036eh		;0358
	ld hl,(0fff6h)		;035b
	push hl			;035e
sub_035fh:
	ld b,(hl)		;035f
	inc hl			;0360
	ld c,000h		;0361
	ld de,00012h		;0363
	jr nz,l0383h		;0366
l0368h:
	bit 6,(hl)		;0368
	jr z,l0370h		;036a
	ld a,c			;036c
	or (hl)			;036d
sub_036eh:
	ld (hl),a		;036e
	inc c			;036f
l0370h:
	add hl,de		;0370
	djnz l0368h		;0371
	pop hl			;0373
	ld b,(hl)		;0374
	inc hl			;0375
l0376h:
	bit 6,(hl)		;0376
	jr nz,l037eh		;0378
	ld a,c			;037a
	or (hl)			;037b
	ld (hl),a		;037c
	inc c			;037d
l037eh:
	add hl,de		;037e
	djnz l0376h		;037f
	jr l038bh		;0381
l0383h:
	pop af			;0383
l0384h:
	ld a,c			;0384
	or (hl)			;0385
	ld (hl),a		;0386
	inc c			;0387
	add hl,de		;0388
	djnz l0384h		;0389
l038bh:
	call 00ee7h		;038b
	call 00357h		;038e
	ld c,000h		;0391
	call 01066h		;0393
	xor a			;0396
	ld c,a			;0397
	ld b,a			;0398
	call l1166h		;0399
l039ch:
	call l113ah		;039c
	ld hl,0fed5h		;039f
	ld (0fdd8h),hl		;03a2
l03a5h:
	call 0130ah		;03a5
	or a			;03a8
l03a9h:
	jp nz,00331h		;03a9
	ld hl,(0fed5h)		;03ac
	ld de,0fe18h		;03af
	or a			;03b2
	sbc hl,de		;03b3
	jr nz,l03a9h		;03b5
	ld a,(0fe66h)		;03b7
	bit 7,a			;03ba
	jr nz,l03e4h		;03bc
	bit 0,a			;03be
	jr z,l03e4h		;03c0
	ld hl,(0fed7h)		;03c2
	ld a,h			;03c5
	cp 0deh			;03c6
	jr c,l03e4h		;03c8
	ld de,l01ffh+1		;03ca
	ld hl,(0fff2h)		;03cd
sub_03d0h:
	add hl,de		;03d0
	ld (0fff2h),hl		;03d1
	call 00357h		;03d4
	ld e,a			;03d7
	ld c,a			;03d8
	dec a			;03d9
	ld (0fe78h),a		;03da
	call 01066h		;03dd
	ld a,h			;03e0
	or l			;03e1
	jr z,l0431h		;03e2
l03e4h:
	ld bc,(0fedbh)		;03e4
	ld b,c			;03e8
	ld c,001h		;03e9
	call sub_0305h		;03eb
	ld hl,(0fed9h)		;03ee
	jp (hl)			;03f1
	ld (0fed7h),de		;03f2
	push bc			;03f6
	ld c,a			;03f7
	call 00357h		;03f8
	call 01066h		;03fb
	pop bc			;03fe
	ld hl,00000h		;03ff
	ld (0fdceh),hl		;0402
l0405h:
	push bc			;0405
	call l113ah		;0406
	call sub_061bh		;0409
	ld hl,(0fed7h)		;040c
	ld (0fdd8h),hl		;040f
	ld de,00080h		;0412
	add hl,de		;0415
	ld (0fed7h),hl		;0416
	call 0130ah		;0419
	pop bc			;041c
	or a			;041d
	jr nz,l0431h		;041e
	inc c			;0420
	ld a,(0ff55h)		;0421
	cp c			;0424
	jr nz,l042eh		;0425
	ld a,001h		;0427
	ld (0fdceh),a		;0429
	ld c,010h		;042c
l042eh:
	djnz l0405h		;042e
	ret			;0430
l0431h:
	call 0034bh		;0431
	dec de			;0434
	dec a			;0435
	inc (hl)		;0436
	jr nz,l0440h		;0437
	ld b,d			;0439
	ld l,a			;043a
	ld l,a			;043b
	ld (hl),h		;043c
	jr nz,l0484h		;043d
	ld (hl),d		;043f
l0440h:
	ld (hl),d		;0440
	ld l,a			;0441
sub_0442h:
	ld (hl),d		;0442
	nop			;0443
	call 00ee7h		;0444
	call l0556h		;0447
	rst 0			;044a
l044bh:
	ex (sp),hl		;044b
	ld a,(hl)		;044c
	or a			;044d
	inc hl			;044e
	ex (sp),hl		;044f
	ret z			;0450
	ld c,a			;0451
	call l0696h		;0452
	jr l044bh		;0455
	xor a			;0457
	ld (0fdefh),a		;0458
	ld (0fdd3h),a		;045b
	ret			;045e
	push de			;045f
	ld e,(hl)		;0460
	inc hl			;0461
	ld d,(hl)		;0462
	ex (sp),hl		;0463
sub_0464h:
	sbc hl,de		;0464
	ex de,hl		;0466
	ex (sp),hl		;0467
	ld (hl),d		;0468
	dec hl			;0469
	ld (hl),e		;046a
	ex (sp),hl		;046b
	pop hl			;046c
	ret			;046d
	in a,(010h)		;046e
	and 002h		;0470
	jr nz,l048bh		;0472
	ld hl,l07d0h		;0474
l0477h:
	push hl			;0477
	ld a,001h		;0478
	call 01642h		;047a
	pop hl			;047d
	in a,(010h)		;047e
	and 002h		;0480
	jr nz,l048bh		;0482
l0484h:
	dec hl			;0484
	ld a,h			;0485
	or l			;0486
	jr nz,l0477h		;0487
	dec a			;0489
	ret			;048a
l048bh:
	ld b,003h		;048b
l048dh:
	push bc			;048d
	ld a,001h		;048e
	call 01635h		;0490
	pop bc			;0493
l0494h:
	in a,(010h)		;0494
	and 002h		;0496
	ret z			;0498
	djnz l048dh		;0499
	ret			;049b
	ld bc,00390h		;049c
	in l,(c)		;049f
	inc b			;04a1
	in h,(c)		;04a2
	ret			;04a4
	ld bc,00320h		;04a5
	out (c),b		;04a8
	in a,(024h)		;04aa
	ld l,a			;04ac
	inc b			;04ad
	out (c),b		;04ae
	in a,(024h)		;04b0
	ld h,a			;04b2
	ret			;04b3
	in a,(0e3h)		;04b4
	ld l,a			;04b6
	in a,(0e4h)		;04b7
	ld h,a			;04b9
	ret			;04ba
	ld a,(0f001h)		;04bb
	or a			;04be
	jr z,l04cah		;04bf
	ld ix,0e808h		;04c1
	ld a,030h		;04c5
	call sub_03d0h		;04c7
l04cah:
	ld ix,0ec08h		;04ca
	ld a,020h		;04ce
	ld b,(ix+000h)		;04d0
	ex af,af'		;04d3
	ld a,(ix-001h)		;04d4
	dec a			;04d7
	add a,a			;04d8
	add a,a			;04d9
	add a,a			;04da
	add a,a			;04db
	add a,a			;04dc
	set 4,a			;04dd
	ex af,af'		;04df
	inc ix			;04e0
	ld de,00012h		;04e2
	push bc			;04e5
	jr l04eah		;04e6
l04e8h:
	add ix,de		;04e8
l04eah:
	djnz l04e8h		;04ea
	pop bc			;04ec
l04edh:
	ld e,(ix+008h)		;04ed
	ld d,(ix+009h)		;04f0
	push bc			;04f3
	ld b,003h		;04f4
l04f6h:
	srl d			;04f6
	rr e			;04f8
	djnz l04f6h		;04fa
	inc de			;04fc
	ld bc,00000h		;04fd
	ex af,af'		;0500
	ld hl,0f000h		;0501
	and 0f0h		;0504
	or (hl)			;0506
	dec (hl)		;0507
	ex af,af'		;0508
	push af			;0509
	call l102dh		;050a
	pop af			;050d
	ld de,0ffeeh		;050e
	add ix,de		;0511
	pop bc			;0513
	djnz l04edh		;0514
	ret			;0516
	ld a,(0e7feh)		;0517
	or a			;051a
	jr z,l052eh		;051b
	ld hl,(0fff6h)		;051d
	ld de,00011h		;0520
l0523h:
	ld b,(hl)		;0523
	inc hl			;0524
l0525h:
	bit 5,(hl)		;0525
	inc hl			;0527
	jr z,l052bh		;0528
	inc (hl)		;052a
l052bh:
	add hl,de		;052b
	djnz l0525h		;052c
l052eh:
	ld ix,0ec08h		;052e
	call sub_0442h		;0532
	ld a,(0f001h)		;0535
	or a			;0538
	ret z			;0539
	ld ix,0e808h		;053a
	call sub_0442h		;053e
	ret			;0541
	ld b,(ix+000h)		;0542
	inc ix			;0545
l0547h:
	push ix			;0547
	pop de			;0549
	push bc			;054a
	ld bc,00012h		;054b
	add ix,bc		;054e
	ld hl,(0fff4h)		;0550
	ld a,(hl)		;0553
	inc a			;0554
	or a			;0555
l0556h:
	sbc hl,bc		;0556
	ld (hl),a		;0558
	ld (0fff4h),hl		;0559
	inc hl			;055c
	ex de,hl		;055d
	ldir			;055e
	pop bc			;0560
	djnz l0547h		;0561
	ret			;0563
	ld de,0ec87h		;0564
	call 0047ch		;0567
sub_056ah:
	ld (0fe59h),hl		;056a
	ld a,(0f001h)		;056d
	or a			;0570
	ret z			;0571
	ld de,0e887h		;0572
	call 0047ch		;0575
	ld (0fe5fh),hl		;0578
	ret			;057b
	ld a,(de)		;057c
	inc de			;057d
	inc a			;057e
l057fh:
	add a,a			;057f
	ld c,a			;0580
	ld b,000h		;0581
	or a			;0583
	sbc hl,bc		;0584
	push hl			;0586
	ex de,hl		;0587
	ldir			;0588
	pop hl			;058a
	ret			;058b
	call sub_193bh		;058c
	ld de,004abh		;058f
l0592h:
	ld a,027h		;0592
	ex de,hl		;0594
	call sub_1170h		;0595
	ld (0fe53h),sp		;0598
	ld sp,hl		;059c
l059dh:
	pop bc			;059d
	out (c),b		;059e
	dec a			;05a0
	jr nz,l059dh		;05a1
	ld sp,(0fe53h)		;05a3
	call sub_117eh		;05a7
	ret			;05aa
	rlca			;05ab
	inc d			;05ac
	rlca			;05ad
	ld b,h			;05ae
	rlca			;05af
	inc de			;05b0
	rlca			;05b1
l05b2h:
	pop bc			;05b2
	rlca			;05b3
	dec b			;05b4
	rlca			;05b5
	ld l,b			;05b6
l05b7h:
	rlca			;05b7
	ld bc,00007h		;05b8
	ld b,014h		;05bb
l05bdh:
	ld b,044h		;05bd
	ld b,013h		;05bf
	ld b,0e1h		;05c1
	ld b,005h		;05c3
l05c5h:
	ld b,0e8h		;05c5
	ld b,001h		;05c7
	ld b,000h		;05c9
	rrca			;05cb
	inc d			;05cc
	rrca			;05cd
l05ceh:
	ld b,h			;05ce
	rrca			;05cf
	inc de			;05d0
	rrca			;05d1
l05d2h:
	pop hl			;05d2
	rrca			;05d3
	dec b			;05d4
	rrca			;05d5
	ret pe			;05d6
	rrca			;05d7
	ld bc,0000fh		;05d8
l05dbh:
	ld c,014h		;05db
	ld c,044h		;05dd
l05dfh:
	ld c,013h		;05df
	ld c,0e1h		;05e1
	ld c,005h		;05e3
	ld c,0e8h		;05e5
l05e7h:
	ld c,001h		;05e7
	ld c,000h		;05e9
	ld (02203h),hl		;05eb
l05eeh:
	rst 8			;05ee
	ld (023e0h),hl		;05ef
	inc bc			;05f2
	inc d			;05f3
	adc a,l			;05f4
	nop			;05f5
	dec b			;05f6
	ex af,af'		;05f7
	dec b			;05f8
	call sub_061bh		;05f9
	jr nz,l060bh		;05fc
	ld hl,0ffbch		;05fe
	ld a,(hl)		;0601
	inc hl			;0602
	inc hl			;0603
	sub (hl)		;0604
	jr z,l060fh		;0605
	xor a			;0607
	dec a			;0608
	jr l060fh		;0609
l060bh:
	in a,(007h)		;060b
	rrca			;060d
	sbc a,a			;060e
l060fh:
	push af			;060f
	ld hl,(0ffe6h)		;0610
l0613h:
	ld a,h			;0613
	or l			;0614
	jr z,l062ah		;0615
	ld a,(0ff98h)		;0617
	or a			;061a
sub_061bh:
	jr nz,l062ah		;061b
	push hl			;061d
	ld hl,l0523h		;061e
	ex (sp),hl		;0621
	jp (hl)			;0622
	ld a,(0ffe4h)		;0623
	cp l			;0626
	call nz,00e98h		;0627
l062ah:
	pop af			;062a
	push af			;062b
	ld hl,0ff98h		;062c
	and (hl)		;062f
	jr nz,l064ah		;0630
	or (hl)			;0632
	jr nz,l0654h		;0633
	inc hl			;0635
	dec (hl)		;0636
	jr nz,l0654h		;0637
	inc hl			;0639
	dec (hl)		;063a
	jr nz,l0654h		;063b
	inc hl			;063d
	dec (hl)		;063e
	jr nz,l0654h		;063f
	dec hl			;0641
	dec hl			;0642
	dec hl			;0643
	dec (hl)		;0644
	call sub_0e88h		;0645
	jr l0654h		;0648
l064ah:
	inc (hl)		;064a
	call 00e94h		;064b
	pop af			;064e
	call sub_056ah		;064f
	xor a			;0652
	ret			;0653
l0654h:
	pop af			;0654
	ret			;0655
	ld hl,0ff98h		;0656
	xor a			;0659
	ld (hl),a		;065a
	inc hl			;065b
	ld (hl),a		;065c
	inc hl			;065d
	ld (hl),a		;065e
	inc hl			;065f
	ld a,(0fdc9h)		;0660
	rra			;0663
	ld (hl),a		;0664
l0665h:
	call 004f9h		;0665
	jr z,l0665h		;0668
	ld a,(0fe58h)		;066a
	or a			;066d
	jr z,l0674h		;066e
	in a,(005h)		;0670
	jr l0683h		;0672
l0674h:
	ld hl,(0ffbch)		;0674
	ld a,(hl)		;0677
	push af			;0678
	ld a,(0ffbch)		;0679
	call 00606h		;067c
	ld (0ffbch),hl		;067f
	pop af			;0682
l0683h:
	or a			;0683
	ret p			;0684
	ld hl,l0592h		;0685
	ld bc,00012h		;0688
	cpir			;068b
	ret nz			;068d
	ld a,080h		;068e
	add a,c			;0690
	ret			;0691
	or d			;0692
	jp 0e4d3h		;0693
l0696h:
	ex (sp),hl		;0696
	jp po,0d2e1h		;0697
	pop de			;069a
	ret nc			;069b
	jp nz,0c0c1h		;069c
	or c			;069f
	call p,0f2f3h		;06a0
	pop af			;06a3
	ld a,(0fe57h)		;06a4
	ld c,a			;06a7
l06a8h:
	in a,(007h)		;06a8
	and 004h		;06aa
	jr z,l06a8h		;06ac
	ld a,c			;06ae
	out (005h),a		;06af
	ret			;06b1
	in a,(006h)		;06b2
l06b4h:
	rrca			;06b4
	sbc a,a			;06b5
	ret			;06b6
	in a,(006h)		;06b7
l06b9h:
	rrca			;06b9
l06bah:
	rrca			;06ba
	jr l06b4h		;06bb
l06bdh:
	call l05b2h		;06bd
	jr z,l06bdh		;06c0
	in a,(004h)		;06c2
	ret			;06c4
l06c5h:
	call l05b7h		;06c5
	jr z,l06c5h		;06c8
	ld a,c			;06ca
l06cbh:
	out (004h),a		;06cb
	ret			;06cd
	in a,(00eh)		;06ce
	jr l06b9h		;06d0
l06d2h:
	call l05ceh		;06d2
	jr z,l06d2h		;06d5
	ld a,c			;06d7
	out (00ch),a		;06d8
	ret			;06da
	in a,(00eh)		;06db
	jr l06b4h		;06dd
l06dfh:
	call l05dbh		;06df
	jr z,l06dfh		;06e2
	in a,(00ch)		;06e4
	ret			;06e6
	in a,(014h)		;06e7
	rlca			;06e9
	rlca			;06ea
	ccf			;06eb
	sbc a,a			;06ec
	ret			;06ed
l06eeh:
	call l05e7h		;06ee
	jr z,l06eeh		;06f1
	ld a,c			;06f3
	out (018h),a		;06f4
	in a,(014h)		;06f6
	call sub_1988h		;06f8
	res 3,a			;06fb
	out (014h),a		;06fd
	set 3,a			;06ff
	ex (sp),hl		;0701
	ex (sp),hl		;0702
	out (014h),a		;0703
	ret			;0705
	push hl			;0706
	push af			;0707
	inc l			;0708
	ld a,(00612h)		;0709
	add a,020h		;070c
	cp l			;070e
	jr nz,l0714h		;070f
	ld hl,0ff9ch		;0711
l0714h:
	pop af			;0714
	cp l			;0715
	jr z,l0719h		;0716
	ex (sp),hl		;0718
l0719h:
	pop hl			;0719
	ret			;071a
	ld a,(0fe58h)		;071b
	or a			;071e
	ret nz			;071f
l0720h:
	in a,(007h)		;0720
	bit 0,a			;0722
	ret z			;0724
	call l062ah		;0725
l0728h:
	jr l0720h		;0728
	push hl			;072a
	call sub_1170h		;072b
	ld a,001h		;072e
l0730h:
	out (007h),a		;0730
	in a,(007h)		;0732
	call sub_117eh		;0734
	and 060h		;0737
	jr z,l0744h		;0739
	bit 5,a			;073b
	ld a,030h		;073d
	out (007h),a		;073f
	jr nz,l0756h		;0741
	or a			;0743
l0744h:
	in a,(005h)		;0744
	jr z,l074ah		;0746
	srl a			;0748
l074ah:
	ld hl,(0ffbeh)		;074a
	ld (hl),a		;074d
	ld a,(0ffbch)		;074e
	call 00606h		;0751
	jr nz,l075dh		;0754
l0756h:
	push bc			;0756
	call 005a4h		;0757
	pop bc			;075a
	pop hl			;075b
	ret			;075c
l075dh:
	ld (0ffbeh),hl		;075d
	pop hl			;0760
	ret			;0761
	ld hl,0fdaah		;0762
	ld b,017h		;0765
	xor a			;0767
l0768h:
	ld (hl),a		;0768
	inc hl			;0769
	djnz l0768h		;076a
	ld hl,00ed7h		;076c
sub_076fh:
	ld bc,0101dh		;076f
l0772h:
	out (01ch),a		;0772
	inc a			;0774
	outi			;0775
	jr nz,l0772h		;0777
	ld a,01fh		;0779
	out (01ch),a		;077b
sub_077dh:
	xor a			;077d
	out (01fh),a		;077e
l0780h:
	ld de,00000h		;0780
	ld bc,00800h		;0783
sub_0786h:
	call 00bf7h		;0786
	call 00bbfh		;0789
	ld a,0c0h		;078c
sub_078eh:
	ld (0fdabh),a		;078e
	ld c,06dh		;0791
	jp l08a3h		;0793
	ld hl,0fdaah		;0796
	ld a,(hl)		;0799
	or a			;079a
sub_079bh:
	jp nz,007c8h		;079b
	ld a,c			;079e
	or a			;079f
	jp m,l06cbh		;07a0
	cp 020h			;07a3
	jp c,l097dh		;07a5
l07a8h:
	ld a,c			;07a8
	ld hl,0fdabh		;07a9
	bit 5,(hl)		;07ac
	jr z,l07bah		;07ae
l07b0h:
	cp 080h			;07b0
	jr nc,l07bah		;07b2
	cp 060h			;07b4
	jr c,l07bah		;07b6
sub_07b8h:
	and 01fh		;07b8
l07bah:
	call l0780h+2		;07ba
	call sub_078eh		;07bd
l07c0h:
	ld hl,0fdafh		;07c0
	ld a,(hl)		;07c3
	cp 04fh			;07c4
	jp c,00776h		;07c6
	jr l0803h		;07c9
	inc hl			;07cb
	bit 4,(hl)		;07cc
	jr z,l07a8h		;07ce
l07d0h:
	inc hl			;07d0
	bit 6,(hl)		;07d1
	jr z,l07d9h		;07d3
	and 001h		;07d5
	ld (hl),a		;07d7
	ret			;07d8
l07d9h:
	bit 0,(hl)		;07d9
	jr z,l07deh		;07db
	cpl			;07dd
l07deh:
	set 7,a			;07de
	call l0780h+2		;07e0
	ld a,(0fdabh)		;07e3
	and 0feh		;07e6
	or (hl)			;07e8
	push hl			;07e9
	call sub_079bh		;07ea
	pop hl			;07ed
	ld (hl),040h		;07ee
	jr l07c0h		;07f0
	ld hl,0fdafh		;07f2
	ld a,(hl)		;07f5
	or a			;07f6
	ret z			;07f7
	ld e,a			;07f8
	xor a			;07f9
	ld d,a			;07fa
	ld (hl),a		;07fb
	ld hl,(0ffe8h)		;07fc
	sbc hl,de		;07ff
	jr l0829h		;0801
l0803h:
	call 006f2h		;0803
	ld a,(0fdaeh)		;0806
	sub 017h		;0809
	jr c,l081eh		;080b
	rra			;080d
	jr nc,l0819h		;080e
	call sub_077dh		;0810
	ret c			;0813
l0814h:
	call 00c2ch		;0814
	jr l0822h		;0817
l0819h:
	call sub_077dh		;0819
	jr c,l0814h		;081c
l081eh:
	ld hl,0fdaeh		;081e
	inc (hl)		;0821
l0822h:
	ld de,00050h		;0822
l0825h:
	ld hl,(0ffe8h)		;0825
	add hl,de		;0828
l0829h:
	ex de,hl		;0829
	ld a,d			;082a
	and 007h		;082b
	ld h,a			;082d
	ld l,e			;082e
	ld (0ffe8h),hl		;082f
	call sub_0e1bh		;0832
	add hl,bc		;0835
	ld c,00eh		;0836
l0838h:
	in a,(01ch)		;0838
	rla			;083a
	jr nc,l0838h		;083b
	ld a,c			;083d
	ld c,01dh		;083e
	out (01ch),a		;0840
	out (c),h		;0842
	inc a			;0844
	out (01ch),a		;0845
	out (c),l		;0847
	ret			;0849
	ld hl,0fdaeh		;084a
	ld a,(hl)		;084d
	cp 018h			;084e
	jr nz,l085ah		;0850
	push hl			;0852
	ld hl,0fdabh		;0853
	bit 7,(hl)		;0856
	pop hl			;0858
	ret nz			;0859
l085ah:
	or a			;085a
	ret z			;085b
	dec (hl)		;085c
	ld de,0ffb0h		;085d
	jr l0825h		;0860
	ld hl,0fdafh		;0862
	ld a,(hl)		;0865
	or a			;0866
	ret z			;0867
	dec (hl)		;0868
	ld hl,(0ffe8h)		;0869
	dec hl			;086c
	jr l0829h		;086d
	ld hl,0fdafh		;086f
	ld a,(hl)		;0872
	cp 04fh			;0873
	ret z			;0875
	inc (hl)		;0876
	ld hl,(0ffe8h)		;0877
	inc hl			;087a
	jr l0829h		;087b
	ld a,(0fdabh)		;087d
	rla			;0880
	ret			;0881
	ld de,(0ffe8h)		;0882
	ld c,a			;0886
	call sub_0b68h		;0887
	ld a,c			;088a
	out (01fh),a		;088b
	ret			;088d
	ld a,(0fdabh)		;088e
	and 00fh		;0891
	jr nz,l089bh		;0893
	ld hl,0fdadh		;0895
	bit 0,(hl)		;0898
	ret nz			;089a
l089bh:
	ld c,a			;089b
	and 00fh		;089c
	jr z,l08a5h		;089e
	ld hl,0fdadh		;08a0
l08a3h:
	res 0,(hl)		;08a3
l08a5h:
	push de			;08a5
	inc de			;08a6
	call 00b60h		;08a7
	pop de			;08aa
	ld a,c			;08ab
	out (01fh),a		;08ac
	ret			;08ae
	push de			;08af
	inc de			;08b0
	call 00b60h		;08b1
	pop de			;08b4
	in a,(01fh)		;08b5
	ret			;08b7
	ld hl,00050h		;08b8
	ld a,(0fdafh)		;08bb
	ld e,a			;08be
	xor a			;08bf
	ld d,a			;08c0
	sbc hl,de		;08c1
	ld de,(0ffe8h)		;08c3
	ret			;08c7
	ld (hl),000h		;08c8
	ld hl,0fdadh		;08ca
	cp 043h			;08cd
	jp z,l09c8h		;08cf
	dec a			;08d2
l08d3h:
	jr nz,l08f4h		;08d3
	ld a,c			;08d5
	cp 052h			;08d6
	jp z,l0d3ch		;08d8
	cp 045h			;08db
	jp z,00dabh		;08dd
	cp 041h			;08e0
	ld hl,0fdabh		;08e2
	jr z,l0958h		;08e5
	cp 047h			;08e7
	jr z,l095bh		;08e9
	ld (0fdb2h),a		;08eb
	ld a,002h		;08ee
l08f0h:
	ld (0fdaah),a		;08f0
	ret			;08f3
l08f4h:
	dec a			;08f4
	jr nz,l0915h		;08f5
	ld a,(0fdb2h)		;08f7
	cp 042h			;08fa
	ld hl,0fdabh		;08fc
	jr z,l095eh		;08ff
	cp 043h			;0901
	jp z,l08d3h		;0903
	cp 055h			;0906
	jp z,l08a3h		;0908
	ld a,c			;090b
	sub 020h		;090c
	ld (0fdbah),a		;090e
	ld a,003h		;0911
	jr l08f0h		;0913
l0915h:
	dec a			;0915
	ld hl,0fdadh		;0916
	jr nz,l0937h		;0919
	ld a,c			;091b
	sub 020h		;091c
	ld (0fdb8h),a		;091e
	ld a,(0fdb2h)		;0921
	cp 02ah			;0924
	jp z,009fch		;0926
	cp 020h			;0929
	jp z,l0a00h		;092b
	cp 03dh			;092e
	jp z,l0940h		;0930
	ld a,004h		;0933
	jr l08f0h		;0935
l0937h:
	dec a			;0937
	jr nz,l0944h		;0938
	ld a,c			;093a
	sub 020h		;093b
	ld (0fdbbh),a		;093d
l0940h:
	ld a,005h		;0940
	jr l08f0h		;0942
l0944h:
	ld a,c			;0944
	sub 020h		;0945
	ld (0fdb9h),a		;0947
	ld a,(0fdb2h)		;094a
	cp 04ch			;094d
	jp z,l0a9bh		;094f
	cp 044h			;0952
	jp z,l0a9fh		;0954
	ret			;0957
l0958h:
	res 5,(hl)		;0958
	ret			;095a
l095bh:
	set 5,(hl)		;095b
	ret			;095d
l095eh:
	ld a,c			;095e
	sub 030h		;095f
	jr z,l0988h		;0961
	dec a			;0963
	jr z,l098bh		;0964
	dec a			;0966
	jr z,l098eh		;0967
	dec a			;0969
	jr z,l0991h		;096a
	dec a			;096c
	jr z,l099fh		;096d
	dec a			;096f
	jr z,l0997h		;0970
	dec a			;0972
	jr z,l09bdh		;0973
	dec a			;0975
	jr z,l0994h		;0976
	dec a			;0978
	jr z,l0983h		;0979
	dec a			;097b
	ret nz			;097c
l097dh:
	ld hl,0fdc2h		;097d
	set 6,(hl)		;0980
	ret			;0982
l0983h:
	set 6,(hl)		;0983
	jp 00e9bh		;0985
l0988h:
	set 0,(hl)		;0988
	ret			;098a
l098bh:
	set 1,(hl)		;098b
	ret			;098d
l098eh:
	set 2,(hl)		;098e
	ret			;0990
l0991h:
	set 3,(hl)		;0991
	ret			;0993
l0994h:
	set 7,(hl)		;0994
	ret			;0996
l0997h:
	set 4,(hl)		;0997
	ld a,040h		;0999
	ld (0fdach),a		;099b
	ret			;099e
l099fh:
	ld a,(0fdb3h)		;099f
	ld c,a			;09a2
	call 008b2h		;09a3
	ld a,(0fdadh)		;09a6
	rla			;09a9
	ret c			;09aa
	ld hl,0fdb3h		;09ab
	ld (hl),c		;09ae
	ret			;09af
l09b0h:
	ld c,020h		;09b0
	call sub_0b83h		;09b2
	ld a,00ah		;09b5
	out (01ch),a		;09b7
	ld a,c			;09b9
	out (01fh),a		;09ba
	ret			;09bc
l09bdh:
	ld a,(hl)		;09bd
	ld hl,0fdadh		;09be
	set 7,(hl)		;09c1
	ld hl,0fdb6h		;09c3
	ld (hl),a		;09c6
	inc hl			;09c7
l09c8h:
	ld a,(0fdb3h)		;09c8
	ld (hl),a		;09cb
	ld hl,(0fdaeh)		;09cc
	ld (0fdb4h),hl		;09cf
	ret			;09d2
	ld a,c			;09d3
	sub 030h		;09d4
	jr z,l0a0fh		;09d6
	dec a			;09d8
	jr z,l0a12h		;09d9
	dec a			;09db
	jr z,l0a15h		;09dc
	dec a			;09de
	jr z,l0a18h		;09df
	dec a			;09e1
	jr z,l09b0h		;09e2
	dec a			;09e4
	jr z,l0a1eh		;09e5
	dec a			;09e7
	jr z,l0a21h		;09e8
	dec a			;09ea
	jr z,l0a1bh		;09eb
	dec a			;09ed
	jr z,l09f8h		;09ee
	dec a			;09f0
	ret nz			;09f1
	ld hl,0fdc2h		;09f2
	res 6,(hl)		;09f5
	ret			;09f7
l09f8h:
	res 6,(hl)		;09f8
	ld hl,(0fdb0h)		;09fa
	ld de,007cah		;09fd
l0a00h:
	add hl,de		;0a00
	ex de,hl		;0a01
	ld b,005h		;0a02
	ld a,020h		;0a04
l0a06h:
	push bc			;0a06
	call sub_0786h		;0a07
sub_0a0ah:
	inc de			;0a0a
	pop bc			;0a0b
	djnz l0a06h		;0a0c
	ret			;0a0e
l0a0fh:
	res 0,(hl)		;0a0f
	ret			;0a11
l0a12h:
	res 1,(hl)		;0a12
	ret			;0a14
l0a15h:
	res 2,(hl)		;0a15
	ret			;0a17
l0a18h:
	res 3,(hl)		;0a18
	ret			;0a1a
l0a1bh:
	res 7,(hl)		;0a1b
	ret			;0a1d
l0a1eh:
	res 4,(hl)		;0a1e
	ret			;0a20
l0a21h:
	ld hl,0fdadh		;0a21
	bit 7,(hl)		;0a24
	jr z,l0a36h		;0a26
	res 7,(hl)		;0a28
	ld hl,0fdb6h		;0a2a
	ld a,(hl)		;0a2d
	ld (0fdabh),a		;0a2e
	inc hl			;0a31
	ld c,(hl)		;0a32
	call l08a3h		;0a33
l0a36h:
	ld hl,(0fdb4h)		;0a36
	ld a,h			;0a39
	ld (0fdb8h),a		;0a3a
	ld a,l			;0a3d
	jr l0a43h		;0a3e
	ld a,(0fdbah)		;0a40
l0a43h:
	or a			;0a43
	ret m			;0a44
	ld hl,00000h		;0a45
	ld b,a			;0a48
	jr z,l0a54h		;0a49
	cp 019h			;0a4b
	ret nc			;0a4d
	ld de,00050h		;0a4e
l0a51h:
	add hl,de		;0a51
	djnz l0a51h		;0a52
l0a54h:
	ld e,a			;0a54
	ld a,(0fdb8h)		;0a55
	or a			;0a58
	ret m			;0a59
	cp 050h			;0a5a
	ret nc			;0a5c
	ld d,a			;0a5d
	ld (0fdaeh),de		;0a5e
	ld c,a			;0a62
	add hl,bc		;0a63
	ld de,(0fdb0h)		;0a64
	jp l0728h		;0a68
	ld a,(0fdafh)		;0a6b
	and 007h		;0a6e
	neg			;0a70
	add a,008h		;0a72
	ld b,a			;0a74
l0a75h:
	push bc			;0a75
	call sub_076fh		;0a76
l0a79h:
	pop bc			;0a79
l0a7ah:
	djnz l0a75h		;0a7a
	ret			;0a7c
	ld hl,l0991h+1		;0a7d
	ld bc,0000eh		;0a80
	cpi			;0a83
	jr nz,l0a8ch		;0a85
	ld a,(hl)		;0a87
	inc hl			;0a88
	ld h,(hl)		;0a89
	ld l,a			;0a8a
	jp (hl)			;0a8b
l0a8ch:
	inc hl			;0a8c
	inc hl			;0a8d
	jp pe,l0983h		;0a8e
	ret			;0a91
	dec c			;0a92
	jp p,l0a06h		;0a93
	ld b,007h		;0a96
	rlca			;0a98
	and h			;0a99
	dec b			;0a9a
l0a9bh:
	ld a,(de)		;0a9b
	adc a,c			;0a9c
	dec bc			;0a9d
	ex af,af'		;0a9e
l0a9fh:
	ld h,d			;0a9f
	rlca			;0aa0
	inc c			;0aa1
	ld l,a			;0aa2
	rlca			;0aa3
	dec bc			;0aa4
	ld c,d			;0aa5
	rlca			;0aa6
	jr l0a79h		;0aa7
	dec bc			;0aa9
	rla			;0aaa
	push de			;0aab
	dec bc			;0aac
	ld e,0c4h		;0aad
	dec bc			;0aaf
	add hl,bc		;0ab0
	ld l,e			;0ab1
	add hl,bc		;0ab2
	dec de			;0ab3
	cp h			;0ab4
	add hl,bc		;0ab5
	ld bc,009c2h		;0ab6
	ld (bc),a		;0ab9
	ret po			;0aba
	add hl,bc		;0abb
	ld a,001h		;0abc
	ld (0fdaah),a		;0abe
	ret			;0ac1
	ld a,043h		;0ac2
	ld (0fdaah),a		;0ac4
	ret			;0ac7
	push bc			;0ac8
	call sub_07b8h		;0ac9
	jr z,l0adbh		;0acc
	ld b,h			;0ace
	ld c,l			;0acf
	ld hl,(0ffe8h)		;0ad0
	dec hl			;0ad3
	add hl,bc		;0ad4
	ld d,h			;0ad5
	ld e,l			;0ad6
	dec hl			;0ad7
	call sub_0c63h		;0ad8
l0adbh:
	pop bc			;0adb
	ld a,c			;0adc
	jp l06bah		;0add
	call sub_07b8h		;0ae0
	ret z			;0ae3
	ld b,h			;0ae4
	ld c,l			;0ae5
	dec bc			;0ae6
	push bc			;0ae7
	ld hl,(0ffe8h)		;0ae8
	push hl			;0aeb
	ld d,h			;0aec
	ld e,l			;0aed
	inc hl			;0aee
	call sub_0c6ch		;0aef
	pop hl			;0af2
	pop bc			;0af3
	add hl,bc		;0af4
	ex de,hl		;0af5
	ld bc,00001h		;0af6
	jp 00bf7h		;0af9
	set 1,(hl)		;0afc
	jr l0b02h		;0afe
	res 1,(hl)		;0b00
l0b02h:
	ld a,(0fdb8h)		;0b02
	ld e,a			;0b05
sub_0b06h:
	ld a,(0fdbah)		;0b06
	ld d,a			;0b09
	ld a,d			;0b0a
	cp 064h			;0b0b
	ret nc			;0b0d
	and 003h		;0b0e
	ld b,a			;0b10
	ld a,d			;0b11
	rra			;0b12
sub_0b13h:
	rra			;0b13
	and 03fh		;0b14
	ld d,a			;0b16
	ld a,e			;0b17
	cp 0a0h			;0b18
	ret nc			;0b1a
	rrca			;0b1b
	ld e,a			;0b1c
	res 7,e			;0b1d
	ld a,001h		;0b1f
	jr c,l0b24h		;0b21
	add a,a			;0b23
l0b24h:
	inc b			;0b24
	jr l0b29h		;0b25
l0b27h:
	add a,a			;0b27
	add a,a			;0b28
l0b29h:
	djnz l0b27h		;0b29
	ld (0fdbch),a		;0b2b
	ld hl,(0fdb0h)		;0b2e
	ld b,d			;0b31
	ld c,e			;0b32
	ld de,00050h		;0b33
	inc b			;0b36
	jr l0b3ah		;0b37
l0b39h:
	add hl,de		;0b39
l0b3ah:
	djnz l0b39h		;0b3a
	ld d,000h		;0b3c
	ld e,c			;0b3e
	add hl,de		;0b3f
	ld a,h			;0b40
	and 007h		;0b41
	ld h,a			;0b43
	ex de,hl		;0b44
	call sub_0b68h		;0b45
	in a,(01fh)		;0b48
	or a			;0b4a
	jp m,00a53h		;0b4b
	cp 020h			;0b4e
	ret nz			;0b50
	ld a,080h		;0b51
	ld b,a			;0b53
	ld hl,0fdadh		;0b54
	ld a,(0fdbch)		;0b57
	or a			;0b5a
	jp p,l0a7ah		;0b5b
	call 007afh		;0b5e
	bit 0,a			;0b61
	jr z,l0b73h		;0b63
	bit 1,(hl)		;0b65
	ret nz			;0b67
sub_0b68h:
	res 0,a			;0b68
l0b6ah:
	push de			;0b6a
	call sub_079bh		;0b6b
	pop de			;0b6e
	ld a,b			;0b6f
	cpl			;0b70
	jr l0b90h		;0b71
l0b73h:
	bit 1,(hl)		;0b73
	ret z			;0b75
	set 0,a			;0b76
	jr l0b6ah		;0b78
	call 007afh		;0b7a
	bit 0,a			;0b7d
	jr nz,l0b95h		;0b7f
	bit 1,(hl)		;0b81
sub_0b83h:
	jr nz,l0b8ch		;0b83
l0b85h:
	ld a,(0fdbch)		;0b85
	cpl			;0b88
	and b			;0b89
	jr l0b90h		;0b8a
l0b8ch:
	ld a,(0fdbch)		;0b8c
	or b			;0b8f
l0b90h:
	set 7,a			;0b90
	jp sub_0786h		;0b92
l0b95h:
	bit 1,(hl)		;0b95
	jr nz,l0b85h		;0b97
	jr l0b8ch		;0b99
	set 1,(hl)		;0b9b
	jr l0ba1h		;0b9d
	res 1,(hl)		;0b9f
l0ba1h:
	ld hl,(0fdbah)		;0ba1
	ld b,l			;0ba4
	ld a,h			;0ba5
	sub l			;0ba6
	ld hl,0fdadh		;0ba7
	set 2,(hl)		;0baa
	jr z,l0bb5h		;0bac
	jr nc,l0bb4h		;0bae
	res 2,(hl)		;0bb0
	neg			;0bb2
l0bb4h:
	inc a			;0bb4
l0bb5h:
	ld e,a			;0bb5
	ld hl,(0fdb8h)		;0bb6
	ld c,l			;0bb9
	ld a,h			;0bba
	sub l			;0bbb
	push bc			;0bbc
	ld hl,0fdadh		;0bbd
	set 3,(hl)		;0bc0
	jr z,l0bcbh		;0bc2
	jr nc,l0bcah		;0bc4
	res 3,(hl)		;0bc6
	neg			;0bc8
l0bcah:
	inc a			;0bca
l0bcbh:
	ld d,a			;0bcb
	push de			;0bcc
	call 00b20h		;0bcd
l0bd0h:
	ld b,h			;0bd0
	ld c,l			;0bd1
	pop hl			;0bd2
	ld a,h			;0bd3
	cp l			;0bd4
	pop de			;0bd5
	ld hl,00000h		;0bd6
	jr c,l0bf3h		;0bd9
	jr nz,l0be0h		;0bdb
	dec hl			;0bdd
	ld b,h			;0bde
	ld c,l			;0bdf
l0be0h:
	call 00b56h		;0be0
	ld a,(0fdb9h)		;0be3
	cp e			;0be6
	ret z			;0be7
	call sub_0b13h		;0be8
	add hl,bc		;0beb
	jr nc,l0be0h		;0bec
	call sub_0b06h		;0bee
	jr l0be0h		;0bf1
l0bf3h:
	call 00b56h		;0bf3
	ld a,(0fdbbh)		;0bf6
	cp d			;0bf9
	ret z			;0bfa
	call sub_0b06h		;0bfb
	add hl,bc		;0bfe
	jr nc,l0bf3h		;0bff
l0c01h:
	call sub_0b13h		;0c01
	jr l0bf3h		;0c04
	push hl			;0c06
	ld hl,0fdadh		;0c07
	bit 2,(hl)		;0c0a
	pop hl			;0c0c
	jr nz,l0c11h		;0c0d
	dec d			;0c0f
	ret			;0c10
l0c11h:
	inc d			;0c11
	ret			;0c12
	push hl			;0c13
	ld hl,0fdadh		;0c14
	bit 3,(hl)		;0c17
l0c19h:
	pop hl			;0c19
	jr nz,l0c1eh		;0c1a
	dec e			;0c1c
	ret			;0c1d
l0c1eh:
	inc e			;0c1e
	ret			;0c1f
	ld hl,00000h		;0c20
	ld a,d			;0c23
	or a			;0c24
	ret z			;0c25
	ld a,e			;0c26
	or a			;0c27
	ret z			;0c28
	inc hl			;0c29
	cp d			;0c2a
	jr c,l0c2fh		;0c2b
	ld a,d			;0c2d
	ld d,e			;0c2e
l0c2fh:
	ld e,000h		;0c2f
l0c31h:
	ld b,h			;0c31
	ld c,l			;0c32
	add a,a			;0c33
	jr nc,l0c37h		;0c34
	inc e			;0c36
l0c37h:
	add hl,bc		;0c37
	jr c,l0c4eh		;0c38
	sub d			;0c3a
	jr nc,l0c4bh		;0c3b
	push af			;0c3d
	ld a,e			;0c3e
	or a			;0c3f
	jr z,l0c47h		;0c40
	pop af			;0c42
	ld e,000h		;0c43
	jr l0c4bh		;0c45
l0c47h:
	pop af			;0c47
	add a,d			;0c48
	jr l0c31h		;0c49
l0c4bh:
	inc hl			;0c4b
	jr l0c31h		;0c4c
l0c4eh:
	sub d			;0c4e
	jr nc,l0c54h		;0c4f
	ld a,e			;0c51
	or a			;0c52
	ret z			;0c53
l0c54h:
	inc hl			;0c54
	ret			;0c55
	push hl			;0c56
	push de			;0c57
sub_0c58h:
	push bc			;0c58
	call sub_0a0ah		;0c59
	pop bc			;0c5c
	pop de			;0c5d
	pop hl			;0c5e
	ret			;0c5f
	ld a,d			;0c60
	and 007h		;0c61
sub_0c63h:
	or 008h			;0c63
	jr l0c6bh		;0c65
	ex de,hl		;0c67
	ld a,d			;0c68
	and 007h		;0c69
l0c6bh:
	ld d,a			;0c6b
sub_0c6ch:
	in a,(01ch)		;0c6c
	rla			;0c6e
	jr nc,sub_0c6ch		;0c6f
	ld a,012h		;0c71
	out (01ch),a		;0c73
	ld a,d			;0c75
	out (01dh),a		;0c76
	ld a,013h		;0c78
	out (01ch),a		;0c7a
	ld a,e			;0c7c
	out (01dh),a		;0c7d
	ld a,01fh		;0c7f
	out (01ch),a		;0c81
l0c83h:
	in a,(01ch)		;0c83
l0c85h:
	rla			;0c85
	jr nc,l0c83h		;0c86
	ret			;0c88
	ld hl,0fdabh		;0c89
	ld a,(hl)		;0c8c
	and 0f0h		;0c8d
	ld (hl),a		;0c8f
	ld hl,(0fdb0h)		;0c90
	ld de,l07d0h		;0c93
	add hl,de		;0c96
	ex de,hl		;0c97
	bit 7,a			;0c98
	push af			;0c9a
	ld bc,00800h		;0c9b
	jr z,l0ca3h		;0c9e
	ld bc,l07b0h		;0ca0
l0ca3h:
	call 00bf7h		;0ca3
	pop af			;0ca6
	jr z,l0cbfh		;0ca7
	ld hl,(0fdb0h)		;0ca9
	ld de,l0780h		;0cac
	add hl,de		;0caf
	ex de,hl		;0cb0
	ld b,050h		;0cb1
l0cb3h:
	inc de			;0cb3
	call 00b60h		;0cb4
	in a,(01fh)		;0cb7
	and 00fh		;0cb9
	jr nz,l0cc4h		;0cbb
	djnz l0cb3h		;0cbd
l0cbfh:
	ld hl,0fdadh		;0cbf
	set 0,(hl)		;0cc2
l0cc4h:
	ld hl,00000h		;0cc4
	ld (0fdaeh),hl		;0cc7
	ld hl,(0fdb0h)		;0cca
	jp l0728h+1		;0ccd
l0cd0h:
	call sub_07b8h		;0cd0
	jr l0cf5h		;0cd3
	ld c,017h		;0cd5
	call sub_077dh		;0cd7
	jr c,l0cddh		;0cda
	inc c			;0cdc
l0cddh:
	ld a,(0fdaeh)		;0cdd
	sub c			;0ce0
	jr nc,l0cd0h		;0ce1
	neg			;0ce3
	ld b,a			;0ce5
	ld hl,00000h		;0ce6
	ld de,00050h		;0ce9
l0cech:
	add hl,de		;0cec
	djnz l0cech		;0ced
	push hl			;0cef
	call sub_07b8h		;0cf0
	pop bc			;0cf3
	add hl,bc		;0cf4
l0cf5h:
	ld b,h			;0cf5
	ld c,l			;0cf6
	push de			;0cf7
	ld hl,(0fdb0h)		;0cf8
	call sub_0c58h		;0cfb
	pop de			;0cfe
	push bc			;0cff
	push de			;0d00
	call sub_0b68h		;0d01
	ld a,020h		;0d04
	out (01fh),a		;0d06
	inc de			;0d08
	dec bc			;0d09
	ld a,b			;0d0a
	or c			;0d0b
	jp nz,l0c01h		;0d0c
	pop de			;0d0f
	pop bc			;0d10
	ld hl,0fdadh		;0d11
	bit 0,(hl)		;0d14
	jr nz,l0d29h		;0d16
	inc de			;0d18
l0d19h:
	call 00b60h		;0d19
	call sub_0b83h		;0d1c
	inc de			;0d1f
	xor a			;0d20
	out (01fh),a		;0d21
	dec bc			;0d23
	ld a,b			;0d24
	or c			;0d25
	jp nz,l0c19h		;0d26
l0d29h:
	jp sub_0b83h		;0d29
	ld hl,(0fdb0h)		;0d2c
	push hl			;0d2f
	call sub_0c58h		;0d30
	pop hl			;0d33
	ld bc,00050h		;0d34
	push bc			;0d37
	push hl			;0d38
	call sub_0e28h		;0d39
l0d3ch:
	pop hl			;0d3c
	pop bc			;0d3d
	ld de,l07d0h		;0d3e
	add hl,de		;0d41
	ex de,hl		;0d42
	ld a,(0fdabh)		;0d43
	rla			;0d46
	jp nc,00bf7h		;0d47
	push bc			;0d4a
	ld hl,0ffb0h		;0d4b
	add hl,de		;0d4e
	push hl			;0d4f
	call sub_0c6ch		;0d50
	pop de			;0d53
	pop bc			;0d54
	jp 00bf7h		;0d55
	ld de,l07d0h		;0d58
	add hl,de		;0d5b
	ex de,hl		;0d5c
	call sub_0b68h		;0d5d
	in a,(01fh)		;0d60
	ret			;0d62
	ld a,b			;0d63
	and 007h		;0d64
	ld b,a			;0d66
	or c			;0d67
	ret z			;0d68
	xor a			;0d69
	jr l0d72h		;0d6a
	ld a,b			;0d6c
	and 007h		;0d6d
	ld b,a			;0d6f
	or c			;0d70
	ret z			;0d71
l0d72h:
	ld (0fdc0h),a		;0d72
	push bc			;0d75
	exx			;0d76
	ex af,af'		;0d77
	ld (0fdbdh),a		;0d78
	ld (0fdbeh),bc		;0d7b
	pop bc			;0d7f
	ex af,af'		;0d80
	exx			;0d81
	ld bc,l1213h		;0d82
l0d85h:
	in a,(01ch)		;0d85
	rla			;0d87
	jr nc,l0d85h		;0d88
	ld a,b			;0d8a
	out (01ch),a		;0d8b
	ld a,h			;0d8d
	and 007h		;0d8e
	out (01dh),a		;0d90
	ld a,c			;0d92
	out (01ch),a		;0d93
	ld a,l			;0d95
	out (01dh),a		;0d96
	ld a,01fh		;0d98
	out (01ch),a		;0d9a
l0d9ch:
	in a,(01ch)		;0d9c
	rla			;0d9e
	jr nc,l0d9ch		;0d9f
	in a,(01fh)		;0da1
	ex af,af'		;0da3
l0da4h:
	in a,(01ch)		;0da4
	rla			;0da6
	jr nc,l0da4h		;0da7
	ld a,b			;0da9
	out (01ch),a		;0daa
	ld a,d			;0dac
	and 007h		;0dad
	out (01dh),a		;0daf
	ld a,c			;0db1
	out (01ch),a		;0db2
	ld a,e			;0db4
	out (01dh),a		;0db5
	ld a,01fh		;0db7
	out (01ch),a		;0db9
l0dbbh:
	in a,(01ch)		;0dbb
	rla			;0dbd
	jr nc,l0dbbh		;0dbe
	ex af,af'		;0dc0
	out (01fh),a		;0dc1
	inc de			;0dc3
	inc hl			;0dc4
	ld a,(0fdadh)		;0dc5
	rra			;0dc8
	jp c,00d0eh		;0dc9
l0dcch:
	in a,(01ch)		;0dcc
	rla			;0dce
	jr nc,l0dcch		;0dcf
	ld a,b			;0dd1
	out (01ch),a		;0dd2
	ld a,h			;0dd4
	and 007h		;0dd5
	or 008h			;0dd7
	out (01dh),a		;0dd9
	ld a,c			;0ddb
	out (01ch),a		;0ddc
	ld a,l			;0dde
	out (01dh),a		;0ddf
	ld a,01fh		;0de1
	out (01ch),a		;0de3
l0de5h:
	in a,(01ch)		;0de5
	rla			;0de7
	jr nc,l0de5h		;0de8
	in a,(01fh)		;0dea
	ex af,af'		;0dec
l0dedh:
	in a,(01ch)		;0ded
	rla			;0def
	jr nc,l0dedh		;0df0
	ld a,b			;0df2
	out (01ch),a		;0df3
	ld a,d			;0df5
	and 007h		;0df6
	or 008h			;0df8
	out (01dh),a		;0dfa
	ld a,c			;0dfc
	out (01ch),a		;0dfd
	ld a,e			;0dff
	out (01dh),a		;0e00
	ld a,01fh		;0e02
	out (01ch),a		;0e04
l0e06h:
	in a,(01ch)		;0e06
	rla			;0e08
	jr nc,l0e06h		;0e09
	ex af,af'		;0e0b
	out (01fh),a		;0e0c
	ld a,(0fdc0h)		;0e0e
	or a			;0e11
	jp nz,l0d19h		;0e12
	dec hl			;0e15
	dec hl			;0e16
	dec de			;0e17
sub_0e18h:
	dec de			;0e18
	exx			;0e19
	dec bc			;0e1a
sub_0e1bh:
	ld a,c			;0e1b
	or a			;0e1c
	call z,sub_061bh	;0e1d
	ld a,c			;0e20
	or b			;0e21
	exx			;0e22
	jp nz,l0c85h		;0e23
	exx			;0e26
	ex af,af'		;0e27
sub_0e28h:
	ld a,(0fdbdh)		;0e28
	ld bc,(0fdbeh)		;0e2b
	ex af,af'		;0e2f
	exx			;0e30
	ld a,h			;0e31
	and 007h		;0e32
	ld h,a			;0e34
	ld a,d			;0e35
	and 007h		;0e36
	ld d,a			;0e38
	jp sub_0b83h		;0e39
	call 006f2h		;0e3c
	ld a,(0fdaeh)		;0e3f
	or a			;0e42
	jr z,l0e6dh		;0e43
	cp 018h			;0e45
	jp z,l0bd0h		;0e47
	cp 00ch			;0e4a
	jr nc,l0e70h		;0e4c
	ld hl,(0ffe8h)		;0e4e
	ld de,00050h		;0e51
	add hl,de		;0e54
	dec hl			;0e55
	ld a,h			;0e56
	and 007h		;0e57
	ld h,a			;0e59
	push hl			;0e5a
	sbc hl,de		;0e5b
	ld a,h			;0e5d
	and 007h		;0e5e
	ld h,a			;0e60
	push hl			;0e61
	call sub_0e1bh		;0e62
	ld b,h			;0e65
	ld c,l			;0e66
	inc bc			;0e67
	pop hl			;0e68
	pop de			;0e69
	call sub_0c63h		;0e6a
l0e6dh:
	jp l0714h		;0e6d
l0e70h:
	ld hl,(0fdb0h)		;0e70
	ld de,(0ffe8h)		;0e73
	or a			;0e77
	sbc hl,de		;0e78
	ld bc,0ff80h		;0e7a
	add hl,bc		;0e7d
	call sub_077dh		;0e7e
	jr nc,l0e87h		;0e81
	ld bc,0ffb0h		;0e83
	add hl,bc		;0e86
l0e87h:
	ld b,h			;0e87
sub_0e88h:
	ld c,l			;0e88
	ld hl,00050h		;0e89
	push hl			;0e8c
	add hl,de		;0e8d
	call sub_0c6ch		;0e8e
	pop bc			;0e91
	push bc			;0e92
	ld hl,l0730h		;0e93
	call sub_077dh		;0e96
	pop bc			;0e99
	jr c,l0ea0h		;0e9a
	ld bc,00050h		;0e9c
	add hl,bc		;0e9f
l0ea0h:
	ex de,hl		;0ea0
	ld hl,(0fdb0h)		;0ea1
	add hl,de		;0ea4
	ex de,hl		;0ea5
	call 00bf7h		;0ea6
	jr l0f12h		;0ea9
	call 006f2h		;0eab
	ld a,(0fdaeh)		;0eae
	cp 00ch			;0eb1
	jr nc,l0ee9h		;0eb3
	call sub_077dh		;0eb5
	jr nc,l0ecah		;0eb8
	ld hl,(0fdb0h)		;0eba
	ld de,l0730h		;0ebd
	add hl,de		;0ec0
	push hl			;0ec1
	ld bc,00050h		;0ec2
	add hl,bc		;0ec5
	pop de			;0ec6
	call sub_0c6ch		;0ec7
l0ecah:
	call sub_0e18h		;0eca
	ld b,h			;0ecd
	ld c,l			;0ece
	push bc			;0ecf
	ld hl,(0fdb0h)		;0ed0
	ld bc,0ffb0h		;0ed3
	call sub_0e28h		;0ed6
	push hl			;0ed9
	ld de,00050h		;0eda
	add hl,de		;0edd
	pop de			;0ede
	pop bc			;0edf
	call sub_0c6ch		;0ee0
	call l075dh		;0ee3
	jp l0bd0h		;0ee6
l0ee9h:
	call sub_0e18h		;0ee9
	ex de,hl		;0eec
	ld hl,00000h		;0eed
	or a			;0ef0
	sbc hl,de		;0ef1
	call sub_077dh		;0ef3
	ld de,l07d0h		;0ef6
	jr nc,l0efeh		;0ef9
	ld de,l0780h		;0efb
l0efeh:
	add hl,de		;0efe
	ld b,h			;0eff
	ld c,l			;0f00
	ld hl,(0fdb0h)		;0f01
	dec de			;0f04
	add hl,de		;0f05
sub_0f06h:
	push hl			;0f06
	ld de,0ffb0h		;0f07
	add hl,de		;0f0a
	pop de			;0f0b
	call sub_0c63h		;0f0c
sub_0f0fh:
	call l0bd0h		;0f0f
l0f12h:
	ld hl,(0ffe8h)		;0f12
	jp l0728h+1		;0f15
	ld hl,(0ffe8h)		;0f18
sub_0f1bh:
	ld bc,(0fdb0h)		;0f1b
	or a			;0f1f
	sbc hl,bc		;0f20
	ret nc			;0f22
l0f23h:
	ld de,00800h		;0f23
	add hl,de		;0f26
	ret			;0f27
	add hl,bc		;0f28
	ld a,h			;0f29
	and 007h		;0f2a
	ld h,a			;0f2c
	ld (0fdb0h),hl		;0f2d
	ld c,00ch		;0f30
	jp 00738h		;0f32
l0f35h:
	push af			;0f35
	ld a,001h		;0f36
	out (01ch),a		;0f38
	pop af			;0f3a
	out (01dh),a		;0f3b
	ret			;0f3d
l0f3eh:
	in a,(01ch)		;0f3e
	bit 5,a			;0f40
	jr nz,l0f3eh		;0f42
l0f44h:
	in a,(01ch)		;0f44
	bit 5,a			;0f46
	jr z,l0f44h		;0f48
	ret			;0f4a
	ld c,a			;0f4b
	jp l08a3h		;0f4c
	inc c			;0f4f
	ld a,(0fdc1h)		;0f50
	ld de,(0fda8h)		;0f53
	jr nz,l0f5ch		;0f57
	xor a			;0f59
	ld d,a			;0f5a
	ld e,a			;0f5b
l0f5ch:
	or a			;0f5c
	ld hl,l07d0h		;0f5d
	sbc hl,de		;0f60
	ret z			;0f62
	ld hl,(0fdb0h)		;0f63
	add hl,de		;0f66
	inc de			;0f67
	ld (0fda8h),de		;0f68
	ex de,hl		;0f6c
	ld l,a			;0f6d
	inc a			;0f6e
	cp 050h			;0f6f
	jr nz,l0f74h		;0f71
	xor a			;0f73
l0f74h:
	ld (0fdc1h),a		;0f74
	call sub_0b68h		;0f77
	in a,(01fh)		;0f7a
	bit 7,a			;0f7c
	jr nz,l0f85h		;0f7e
	or a			;0f80
	ret nz			;0f81
	ld c,a			;0f82
	inc c			;0f83
sub_0f84h:
	ret			;0f84
l0f85h:
	ld a,020h		;0f85
	ret			;0f87
	ld a,(0fe79h)		;0f88
	or a			;0f8b
	ret nz			;0f8c
	call 00e3eh		;0f8d
	ld a,000h		;0f90
	jr l0f35h		;0f92
	ld a,050h		;0f94
	jr l0f35h		;0f96
	ld (0ffe4h),hl		;0f98
	ld a,(0fdabh)		;0f9b
	rla			;0f9e
sub_0f9fh:
	ret nc			;0f9f
	rla			;0fa0
	ret nc			;0fa1
	ld hl,(0fdb0h)		;0fa2
	ld de,007cah		;0fa5
	add hl,de		;0fa8
	ex de,hl		;0fa9
	ld hl,0ffe5h		;0faa
	ld a,030h		;0fad
	call 00ebfh		;0faf
	ld a,03ah		;0fb2
	call sub_0786h		;0fb4
	xor a			;0fb7
	call sub_079bh		;0fb8
	inc de			;0fbb
	dec hl			;0fbc
sub_0fbdh:
	ld a,030h		;0fbd
	rld			;0fbf
	call l0ecah+2		;0fc1
sub_0fc4h:
	rld			;0fc4
	call l0ecah+2		;0fc6
	rld			;0fc9
	ret			;0fcb
	push af			;0fcc
	call sub_0786h		;0fcd
	xor a			;0fd0
	call sub_079bh		;0fd1
	pop af			;0fd4
	inc de			;0fd5
	ret			;0fd6
	ld l,d			;0fd7
	ld d,b			;0fd8
	ld d,(hl)		;0fd9
	sbc a,c			;0fda
	add hl,de		;0fdb
	ld a,(bc)		;0fdc
	add hl,de		;0fdd
	add hl,de		;0fde
	ld a,b			;0fdf
	rrca			;0fe0
	ld l,l			;0fe1
	rrca			;0fe2
	nop			;0fe3
	nop			;0fe4
	nop			;0fe5
	nop			;0fe6
	in a,(014h)		;0fe7
	res 4,a			;0fe9
	ld c,a			;0feb
	ld a,(0fe68h)		;0fec
	or a			;0fef
	ld a,c			;0ff0
	jr nz,l0ff9h		;0ff1
	push hl			;0ff3
	ld hl,0fe65h		;0ff4
	or (hl)			;0ff7
	pop hl			;0ff8
l0ff9h:
	push bc			;0ff9
	push af			;0ffa
	pop bc			;0ffb
	call sub_1988h		;0ffc
	ld b,a			;0fff
	push bc			;1000
	pop af			;1001
	pop bc			;1002
	out (014h),a		;1003
	ret			;1005
	in a,(014h)		;1006
	bit 4,a			;1008
	ret nz			;100a
	set 4,a			;100b
	jr l0ff9h		;100d
	in a,(014h)		;100f
	jr nz,l1017h		;1011
	set 2,a			;1013
	jr l1019h		;1015
l1017h:
	res 2,a			;1017
l1019h:
	jr l0ff9h		;1019
	ld b,a			;101b
	in a,(014h)		;101c
	and 0dfh		;101e
	or b			;1020
	jr l0ff9h		;1021
	call sub_0f06h		;1023
	ret nz			;1026
	call 00fa7h		;1027
l102ah:
	call l0f3eh+1		;102a
l102dh:
	jr nz,l102ah		;102d
l102fh:
	call l0f3eh+1		;102f
	jr z,l102fh		;1032
l1034h:
	call l0f3eh+1		;1034
	jr nz,l1034h		;1037
	ld a,032h		;1039
	call 01635h		;103b
	ret			;103e
	ld a,001h		;103f
	call 01642h		;1041
	in a,(010h)		;1044
	and 002h		;1046
	ret			;1048
	ld hl,0fdeeh		;1049
	ld a,(hl)		;104c
	ld (hl),000h		;104d
	or a			;104f
	push bc			;1050
	push de			;1051
	call nz,01423h		;1052
	pop de			;1055
	call 00ee7h		;1056
	pop bc			;1059
	ld a,(0fe66h)		;105a
	rrca			;105d
sub_105eh:
	ret nc			;105e
	call sub_0f9fh		;105f
	ret z			;1062
	ld a,0c8h		;1063
	out (086h),a		;1065
	ld hl,0fe5bh		;1067
	call 00f97h		;106a
	inc hl			;106d
	call sub_0f84h		;106e
	ld a,0d0h		;1071
	out (086h),a		;1073
	ld hl,0fe61h		;1075
	call 00f97h		;1078
	inc hl			;107b
	call sub_0f84h		;107c
	ld a,0c0h		;107f
	out (086h),a		;1081
	ret			;1083
l1084h:
	call sub_0fbdh		;1084
	bit 6,a			;1087
	ret z			;1089
	bit 4,a			;108a
	jr z,l1084h		;108c
	ld a,(hl)		;108e
	or 070h			;108f
	out (087h),a		;1091
	call sub_0fbdh		;1093
	ret			;1096
	ld a,(hl)		;1097
	out (084h),a		;1098
	inc hl			;109a
	ld a,(hl)		;109b
	out (085h),a		;109c
	ret			;109e
	in a,(086h)		;109f
	rrca			;10a1
	rrca			;10a2
	rrca			;10a3
	and 003h		;10a4
	ret			;10a6
	ld a,0d0h		;10a7
	out (010h),a		;10a9
	ex (sp),hl		;10ab
	ex (sp),hl		;10ac
	ex (sp),hl		;10ad
	ex (sp),hl		;10ae
	ex (sp),hl		;10af
	ex (sp),hl		;10b0
	ex (sp),hl		;10b1
	ex (sp),hl		;10b2
	jr l10b6h		;10b3
	halt			;10b5
l10b6h:
	in a,(010h)		;10b6
	bit 0,a			;10b8
	jr nz,l10b6h		;10ba
	ret			;10bc
l10bdh:
	in a,(087h)		;10bd
	rlca			;10bf
	jr c,l10bdh		;10c0
	rrca			;10c2
	ret			;10c3
	push hl			;10c4
	and 003h		;10c5
	call 0111dh		;10c7
	xor a			;10ca
	call sub_0f0fh		;10cb
	xor a			;10ce
	call sub_0f1bh		;10cf
	call l0f23h		;10d2
	call sub_15c5h		;10d5
	call 0166eh		;10d8
	pop hl			;10db
	jr nz,l1114h		;10dc
	ld a,(0fdc6h)		;10de
	ld c,a			;10e1
	dec c			;10e2
	ld a,008h		;10e3
	scf			;10e5
	ret z			;10e6
sub_10e7h:
	ld b,000h		;10e7
	dec c			;10e9
	jr z,l10f4h		;10ea
	ld a,(0fe78h)		;10ec
	or a			;10ef
	jr nz,l1111h		;10f0
	ld b,003h		;10f2
l10f4h:
	push bc			;10f4
	ld a,001h		;10f5
	or a			;10f7
	call sub_0f0fh		;10f8
	call 0166eh		;10fb
	pop bc			;10fe
	jr nz,l110eh		;10ff
	ld a,(0fdc5h)		;1101
	cp 00ah			;1104
	jr c,l110eh		;1106
	inc b			;1108
	cp 014h			;1109
	jr c,l110eh		;110b
	inc b			;110d
l110eh:
	ld a,b			;110e
	scf			;110f
	ret			;1110
l1111h:
	scf			;1111
	ccf			;1112
	ret			;1113
l1114h:
	ld a,020h		;1114
	call sub_0f1bh		;1116
	call 0166eh		;1119
	jr nz,l1111h		;111c
	ld a,(0fdc6h)		;111e
	or a			;1121
	ld c,a			;1122
	ld a,006h		;1123
	scf			;1125
	ret z			;1126
	inc a			;1127
	dec c			;1128
	scf			;1129
l112ah:
	ret z			;112a
	jr l1111h		;112b
	push de			;112d
	push bc			;112e
	ld hl,(0fff6h)		;112f
	call sub_105eh		;1132
	ld (0fff6h),hl		;1135
	inc hl			;1138
	ld (hl),a		;1139
l113ah:
	inc hl			;113a
	ex af,af'		;113b
	ld (hl),a		;113c
	ex af,af'		;113d
	xor a			;113e
	ld b,008h		;113f
l1141h:
	inc hl			;1141
	ld (hl),a		;1142
	djnz l1141h		;1143
	ld de,0fed5h		;1145
	inc hl			;1148
	ld (hl),e		;1149
	inc hl			;114a
	ld (hl),d		;114b
	ld de,0ff55h		;114c
	inc hl			;114f
	ld (hl),e		;1150
	inc hl			;1151
	ld (hl),d		;1152
	pop de			;1153
	inc hl			;1154
	ld (hl),e		;1155
	inc hl			;1156
	ld (hl),d		;1157
	pop de			;1158
	inc hl			;1159
	ld (hl),e		;115a
	inc hl			;115b
	ld (hl),d		;115c
	ret			;115d
	ld c,(hl)		;115e
	inc c			;115f
	ld de,0ffeeh		;1160
	add hl,de		;1163
	ld (hl),c		;1164
	ret			;1165
l1166h:
	push de			;1166
	ld hl,(0fff6h)		;1167
	ld b,(hl)		;116a
l116bh:
	inc hl			;116b
	ld de,00012h		;116c
l116fh:
	ld a,(hl)		;116f
sub_1170h:
	and 00fh		;1170
	cp c			;1172
	jr z,l117dh		;1173
	add hl,de		;1175
	djnz l116fh		;1176
	pop de			;1178
l1179h:
	ld hl,00000h		;1179
	ret			;117c
l117dh:
	ld a,(hl)		;117d
sub_117eh:
	rrca			;117e
	rrca			;117f
	rrca			;1180
	rrca			;1181
	and 007h		;1182
	ld (0fdd0h),a		;1184
	inc hl			;1187
	pop de			;1188
sub_1189h:
	bit 0,e			;1189
	jr nz,l11a7h		;118b
	bit 2,a			;118d
	jr z,l11a7h		;118f
	bit 6,(hl)		;1191
	jr nz,l11a7h		;1193
	push af			;1195
	call l112ah		;1196
	pop af			;1199
	push hl			;119a
	call sub_0fc4h		;119b
	pop hl			;119e
	jr nc,l1179h		;119f
	ld c,a			;11a1
	ld a,(hl)		;11a2
	and 0e0h		;11a3
	or c			;11a5
	ld (hl),a		;11a6
l11a7h:
	ld a,(hl)		;11a7
	ld (0fdd1h),a		;11a8
	inc hl			;11ab
	push hl			;11ac
	bit 4,a			;11ad
	jr z,l11bbh		;11af
	ld hl,(0fff4h)		;11b1
	ld b,a			;11b4
	ld a,(hl)		;11b5
	dec a			;11b6
	sub b			;11b7
	inc hl			;11b8
	jr l11beh		;11b9
l11bbh:
	ld hl,l16b2h		;11bb
l11beh:
	ld de,00012h		;11be
	and 00fh		;11c1
	ld b,a			;11c3
	inc b			;11c4
	jr l11c8h		;11c5
l11c7h:
	add hl,de		;11c7
l11c8h:
	djnz l11c7h		;11c8
	ld e,(hl)		;11ca
	inc hl			;11cb
	ld d,(hl)		;11cc
	inc hl			;11cd
	ld (0fde5h),de		;11ce
	ex (sp),hl		;11d2
	ld (hl),e		;11d3
	inc hl			;11d4
	ld (hl),d		;11d5
	dec hl			;11d6
	ex (sp),hl		;11d7
	ld a,(hl)		;11d8
	ld (0fdd2h),a		;11d9
	inc hl			;11dc
	ld de,0ff55h		;11dd
	ld bc,0000fh		;11e0
	ldir			;11e3
	pop hl			;11e5
	ret			;11e6
	ld b,a			;11e7
	inc b			;11e8
	ld hl,0fe69h		;11e9
	ld de,00003h		;11ec
	jr l11f2h		;11ef
l11f1h:
	add hl,de		;11f1
l11f2h:
	djnz l11f1h		;11f2
	ld de,0fe75h		;11f4
	ld bc,00003h		;11f7
	ldir			;11fa
	ld b,a			;11fc
	inc b			;11fd
	ld a,036h		;11fe
	jr l1204h		;1200
l1202h:
	rrca			;1202
	rrca			;1203
l1204h:
	djnz l1202h		;1204
	ld hl,0fe65h		;1206
	and (hl)		;1209
	ld c,a			;120a
	in a,(014h)		;120b
	ld b,a			;120d
	and (hl)		;120e
	cp c			;120f
	ret z			;1210
	ld a,(hl)		;1211
	cpl			;1212
l1213h:
	and b			;1213
	or c			;1214
	call sub_1988h		;1215
	out (014h),a		;1218
	xor a			;121a
	dec a			;121b
	ret			;121c
	call sub_10e7h		;121d
	ret z			;1220
	ld a,(0fe76h)		;1221
	call 01642h		;1224
	xor a			;1227
	dec a			;1228
	ret			;1229
	xor a			;122a
	ld (0fdceh),a		;122b
	ld a,(0fdeeh)		;122e
	or a			;1231
	ret nz			;1232
	ld (0fdefh),a		;1233
	ret			;1236
	ld h,b			;1237
	ld l,c			;1238
	ret			;1239
	ld a,c			;123a
	ld (0fdcdh),a		;123b
	ld (0fde8h),a		;123e
	ld a,(0fdd2h)		;1241
	and 003h		;1244
	ld b,a			;1246
	xor a			;1247
	ld l,c			;1248
	ld h,a			;1249
	inc b			;124a
	jr l1251h		;124b
l124dh:
	srl l			;124d
	scf			;124f
	rla			;1250
l1251h:
	djnz l124dh		;1251
	and c			;1253
	ld (0fdcbh),a		;1254
	ld de,(0fde5h)		;1257
	ld a,e			;125b
	or d			;125c
	jr z,l1261h		;125d
	add hl,de		;125f
	ld l,(hl)		;1260
l1261h:
	ld a,l			;1261
	ld (0fdcch),a		;1262
	ret			;1265
	ld (0fdceh),bc		;1266
	ret			;126a
	ld (0fdd8h),bc		;126b
sub_126fh:
	ret			;126f
	push af			;1270
	push bc			;1271
	ld a,i			;1272
	push af			;1274
	pop bc			;1275
	ld a,c			;1276
	ld (0fe56h),a		;1277
	pop bc			;127a
	pop af			;127b
	di			;127c
	ret			;127d
	push af			;127e
	ld a,(0fe56h)		;127f
	bit 2,a			;1282
	jr z,l1287h		;1284
	ei			;1286
l1287h:
	pop af			;1287
	ret			;1288
	ld (0f001h),a		;1289
	ld (0f000h),a		;128c
	ld hl,0fe66h		;128f
	ld (hl),a		;1292
	dec hl			;1293
	ld (hl),003h		;1294
	call sub_12dbh		;1296
	ld hl,0012ch		;1299
l129ch:
	push hl			;129c
	ld a,001h		;129d
	call 01635h		;129f
	pop hl			;12a2
	in a,(087h)		;12a3
	rlca			;12a5
	jr nc,l12aeh		;12a6
	dec hl			;12a8
	ld a,h			;12a9
	or l			;12aa
	jr nz,l129ch		;12ab
	ret			;12ad
l12aeh:
	in a,(083h)		;12ae
	ld c,a			;12b0
	cpl			;12b1
	out (083h),a		;12b2
	in a,(083h)		;12b4
	cp c			;12b6
	ret z			;12b7
	cpl			;12b8
	sub c			;12b9
	ret nz			;12ba
	ld hl,0fe66h		;12bb
	set 0,(hl)		;12be
	call sub_12dbh		;12c0
	ld a,001h		;12c3
	call 01635h		;12c5
	in a,(087h)		;12c8
	ld hl,0fe65h		;12ca
	rlca			;12cd
	jr nc,l12d2h		;12ce
	res 1,(hl)		;12d0
l12d2h:
	call sub_0fbdh		;12d2
	ld a,0c8h		;12d5
l12d7h:
	out (086h),a		;12d7
	ld b,0c8h		;12d9
sub_12dbh:
	call sub_126fh		;12db
	jp z,01230h		;12de
	ld hl,0ec00h		;12e1
	ld a,008h		;12e4
	call 01235h		;12e6
	jr nc,l12f3h		;12e9
	ld a,010h		;12eb
	call 01235h		;12ed
	jp c,01230h		;12f0
l12f3h:
	ld hl,0ec00h		;12f3
	ld de,0fe5bh		;12f6
	call 01298h		;12f9
	ret nz			;12fc
	ld hl,0fe66h		;12fd
	set 7,(hl)		;1300
	ld a,0d0h		;1302
	out (086h),a		;1304
	ld b,014h		;1306
	call sub_126fh		;1308
	ret z			;130b
	ld hl,0e800h		;130c
	ld a,008h		;130f
	call 01235h		;1311
	jr nc,l131ch		;1314
	ld a,010h		;1316
	call 01235h		;1318
	ret c			;131b
l131ch:
	ld a,0c9h		;131c
	out (086h),a		;131e
	ld hl,0e800h		;1320
	ld de,0fe61h		;1323
	call 01298h		;1326
	ret nz			;1329
	ld a,0ffh		;132a
	ld (0f001h),a		;132c
	ret			;132f
	xor a			;1330
	ld (0fe66h),a		;1331
	ret			;1334
	out (083h),a		;1335
	cp 008h			;1337
	in a,(086h)		;1339
	set 6,a			;133b
	res 5,a			;133d
	jr z,l1345h		;133f
	res 6,a			;1341
	set 5,a			;1343
l1345h:
	out (086h),a		;1345
	xor a			;1347
	out (081h),a		;1348
	out (084h),a		;134a
	out (085h),a		;134c
	inc a			;134e
	out (082h),a		;134f
	ld a,020h		;1351
	out (087h),a		;1353
	call sub_0fbdh		;1355
	rrca			;1358
	ret c			;1359
	ld bc,00080h		;135a
	in a,(086h)		;135d
	and 060h		;135f
	cp 020h			;1361
	jr z,l1369h		;1363
	inir			;1365
	inir			;1367
l1369h:
	inir			;1369
	inir			;136b
	or a			;136d
	ret			;136e
l136fh:
	push bc			;136f
	ld a,00ah		;1370
	call 01635h		;1372
	pop bc			;1375
	in a,(087h)		;1376
	bit 6,a			;1378
	jr nz,l137fh		;137a
	djnz l136fh		;137c
	ret			;137e
l137fh:
	ld a,010h		;137f
	out (087h),a		;1381
	ld b,0fah		;1383
l1385h:
	push bc			;1385
	ld a,00ah		;1386
	call 01635h		;1388
	pop bc			;138b
	in a,(087h)		;138c
	rlca			;138e
	jr nc,l1395h		;138f
	djnz l1385h		;1391
	xor a			;1393
	ret			;1394
l1395h:
	xor a			;1395
	dec a			;1396
	ret			;1397
	push de			;1398
	push hl			;1399
	xor a			;139a
	ld b,a			;139b
	ld c,002h		;139c
l139eh:
	add a,(hl)		;139e
	inc hl			;139f
	djnz l139eh		;13a0
	dec c			;13a2
	jr nz,l139eh		;13a3
	or a			;13a5
	pop hl			;13a6
	jr nz,l13d0h		;13a7
	push hl			;13a9
	pop ix			;13aa
	ld de,00090h		;13ac
	ld b,003h		;13af
l13b1h:
	ld a,(de)		;13b1
	sub (hl)		;13b2
	jr nz,l13d0h		;13b3
	inc hl			;13b5
	inc de			;13b6
	djnz l13b1h		;13b7
	ld hl,0f000h		;13b9
	ld a,(ix+008h)		;13bc
	add a,(hl)		;13bf
	ld (hl),a		;13c0
	push ix			;13c1
	pop hl			;13c3
	ld de,00003h		;13c4
	add hl,de		;13c7
	xor a			;13c8
l13c9h:
	pop de			;13c9
	ld bc,00004h		;13ca
	ldir			;13cd
	ret			;13cf
l13d0h:
	ld hl,l12d7h		;13d0
	xor a			;13d3
	dec a			;13d4
	jr l13c9h		;13d5
	ld sp,00001h		;13d7
	nop			;13da
	in a,(014h)		;13db
	call sub_1988h		;13dd
	push af			;13e0
	xor 002h		;13e1
sub_13e3h:
	out (014h),a		;13e3
	ld a,005h		;13e5
	call 01635h		;13e7
	pop af			;13ea
	out (014h),a		;13eb
sub_13edh:
	ret			;13ed
l13eeh:
	xor a			;13ee
	scf			;13ef
	jr l13f3h		;13f0
l13f2h:
	xor a			;13f2
l13f3h:
	ld hl,(0fdd8h)		;13f3
	ld bc,08088h		;13f6
	push af			;13f9
	ld a,(0fdcch)		;13fa
	out (089h),a		;13fd
	ld a,(0fdceh)		;13ff
	out (08ah),a		;1402
	pop af			;1404
	call 0fe7ah		;1405
	xor a			;1408
	ret			;1409
	call 01407h		;140a
	jr z,l13eeh		;140d
	dec a			;140f
	call z,sub_13edh	;1410
	xor a			;1413
	ld (hl),a		;1414
	inc a			;1415
	inc hl			;1416
	ld (hl),a		;1417
	inc hl			;1418
	ld (hl),a		;1419
	inc a			;141a
	inc hl			;141b
	ld (hl),a		;141c
	jr l1472h		;141d
	call 01407h		;141f
	jr z,l13f2h		;1422
	dec a			;1424
	call z,013f1h		;1425
	push hl			;1428
	inc hl			;1429
	xor a			;142a
	ld (hl),a		;142b
	inc hl			;142c
	inc hl			;142d
	ld (hl),c		;142e
	ld a,c			;142f
	cp 002h			;1430
	pop hl			;1432
	jr nz,l1441h		;1433
	ld a,(0ff58h)		;1435
	inc a			;1438
	ld (hl),a		;1439
	push hl			;143a
	call 013d9h		;143b
	ldir			;143e
	pop hl			;1440
l1441h:
	ld a,(hl)		;1441
	or a			;1442
	jr z,l146ah		;1443
	dec (hl)		;1445
	call 013d9h		;1446
	ld a,(de)		;1449
	cpi			;144a
	jr nz,l146ah		;144c
	inc de			;144e
	jp pe,01349h		;144f
	ld hl,0fddah		;1452
	inc (hl)		;1455
	ld a,(0ff55h)		;1456
	cp (hl)			;1459
	jr nz,l1464h		;145a
	ld (hl),000h		;145c
	inc hl			;145e
	inc (hl)		;145f
	jr nz,l1464h		;1460
	inc hl			;1462
	inc (hl)		;1463
l1464h:
	xor a			;1464
	ld (0fdd5h),a		;1465
	jr l1472h		;1468
l146ah:
	xor a			;146a
	ld (0fdd3h),a		;146b
	inc a			;146e
	ld (0fdd5h),a		;146f
l1472h:
	xor a			;1472
	ld (0fdd7h),a		;1473
	ld hl,0fdefh		;1476
	ld a,(hl)		;1479
	ld (hl),001h		;147a
	or a			;147c
	jr z,l1494h		;147d
	call sub_13e3h		;147f
	ld a,(de)		;1482
	cpi			;1483
	jr nz,l148dh		;1485
	inc de			;1487
	jp pe,01382h		;1488
	jr l14a4h		;148b
l148dh:
	ld a,(0fdeeh)		;148d
	or a			;1490
	call nz,01423h		;1491
l1494h:
	call sub_13e3h		;1494
	ldir			;1497
	ld a,(0fdd5h)		;1499
	or a			;149c
	call nz,01420h		;149d
	xor a			;14a0
	ld (0fdeeh),a		;14a1
l14a4h:
	ld hl,0fdcbh		;14a4
	ld b,(hl)		;14a7
	inc b			;14a8
	ld de,00080h		;14a9
	ld hl,(0fff2h)		;14ac
	jr l14b2h		;14af
l14b1h:
	add hl,de		;14b1
l14b2h:
	djnz l14b1h		;14b2
	ld b,d			;14b4
	ld c,e			;14b5
	ld de,(0fdd8h)		;14b6
	ld a,(0fdd4h)		;14ba
	or a			;14bd
	jr nz,l14c5h		;14be
	inc a			;14c0
	ld (0fdeeh),a		;14c1
	ex de,hl		;14c4
l14c5h:
	call 0fe7ah		;14c5
l14c8h:
	ld a,(0fdd6h)		;14c8
	cp 001h			;14cb
	ld a,(0fdd7h)		;14cd
	ret nz			;14d0
	or a			;14d1
	ret nz			;14d2
	ld (0fdeeh),a		;14d3
	jp 01423h		;14d6
	ld hl,0fdcdh		;14d9
	ld de,0fddah		;14dc
	ld bc,00004h		;14df
	ret			;14e2
	ld hl,0fdcch		;14e3
	ld de,0fde7h		;14e6
	ld bc,00007h		;14e9
	ret			;14ec
	ld b,000h		;14ed
	jr l14f3h		;14ef
	ld b,0ffh		;14f1
l14f3h:
	ld a,(0fdd2h)		;14f3
	and 003h		;14f6
	ret nz			;14f8
	pop hl			;14f9
	ld hl,0fdcch		;14fa
	ld de,(0fdd8h)		;14fd
	sub 001h		;1501
	ld a,b			;1503
	jp 0fe7ah		;1504
	ld hl,0fdd0h		;1507
	ld a,(hl)		;150a
	or a			;150b
	inc hl			;150c
	inc hl			;150d
	inc hl			;150e
	ret			;150f
l1510h:
	ld a,(0fe55h)		;1510
	ld hl,0fde7h		;1513
	ld de,(0fff2h)		;1516
	call 0fdf0h		;151a
	jp l1543h		;151d
	xor a			;1520
	jr l1525h		;1521
	ld a,0ffh		;1523
l1525h:
	ld (0fe55h),a		;1525
	ld a,(0fdebh)		;1528
	cp 001h			;152b
l152dh:
	jr z,l1510h		;152d
	push af			;152f
	call sub_17b7h		;1530
	pop af			;1533
	bit 2,a			;1534
	jp nz,l14c8h		;1536
	call sub_0f9fh		;1539
	dec a			;153c
	ld c,a			;153d
	ld a,(0fde4h)		;153e
	cp c			;1541
	push af			;1542
l1543h:
	inc a			;1543
	add a,a			;1544
	add a,a			;1545
	add a,a			;1546
	ld c,a			;1547
	ld a,(0fddfh)		;1548
	or c			;154b
	ld c,a			;154c
	ld a,(0fdedh)		;154d
	dec a			;1550
	and 003h		;1551
	ld e,a			;1553
	rlca			;1554
	rlca			;1555
	rlca			;1556
	rlca			;1557
	rlca			;1558
	or c			;1559
	or 080h			;155a
	out (086h),a		;155c
	ld hl,0fde2h		;155e
	call 00f97h		;1561
	ld a,(0fddeh)		;1564
	out (083h),a		;1567
	pop af			;1569
	jr z,l1580h		;156a
	or a			;156c
	ld hl,0fe5eh		;156d
	jr z,l1575h		;1570
	ld hl,0fe64h		;1572
l1575h:
	ld a,(hl)		;1575
	out (081h),a		;1576
	dec hl			;1578
	call sub_0f84h		;1579
	ld a,001h		;157c
	out (082h),a		;157e
l1580h:
	call sub_0fbdh		;1580
	bit 4,a			;1583
sub_1585h:
	jr z,l1580h		;1585
	ld hl,(0fff2h)		;1587
	ld bc,00080h		;158a
	ld a,(0fe55h)		;158d
	or a			;1590
	jr z,l15ach		;1591
	ld a,030h		;1593
	out (087h),a		;1595
l1597h:
	in a,(087h)		;1597
	bit 3,a			;1599
	jr z,l1597h		;159b
	ld a,e			;159d
	cp 002h			;159e
	jr nz,l15a6h		;15a0
	otir			;15a2
	otir			;15a4
l15a6h:
	otir			;15a6
	otir			;15a8
	jr l15c0h		;15aa
l15ach:
	ld a,020h		;15ac
	out (087h),a		;15ae
	call sub_0fbdh		;15b0
	ld a,e			;15b3
	cp 002h			;15b4
	jr nz,l15bch		;15b6
	inir			;15b8
	inir			;15ba
l15bch:
	inir			;15bc
	inir			;15be
l15c0h:
	call sub_0fbdh		;15c0
	and 001h		;15c3
sub_15c5h:
	jp l1543h		;15c5
	call 01560h		;15c8
	ld a,(0fde4h)		;15cb
	call 0111dh		;15ce
	push af			;15d1
	call l0f23h		;15d2
	ld a,(0fddfh)		;15d5
	or a			;15d8
	call sub_0f0fh		;15d9
	ld a,(0fdedh)		;15dc
	and 020h		;15df
	call sub_0f1bh		;15e1
	pop af			;15e4
	call nz,0166eh		;15e5
	jr nz,l15edh		;15e8
	call sub_1585h		;15ea
l15edh:
	jp nz,l1543h		;15ed
	ld b,005h		;15f0
l15f2h:
	push bc			;15f2
	call sub_1170h		;15f3
	ld a,(0fdedh)		;15f6
	and 003h		;15f9
	ld e,a			;15fb
	ld a,(0fddeh)		;15fc
sub_15ffh:
	out (012h),a		;15ff
	ld a,(0fde0h)		;1601
	out (011h),a		;1604
	ld hl,(0fff2h)		;1606
	ld bc,00013h		;1609
	call 0154fh		;160c
	ld a,(0fe55h)		;160f
	or a			;1612
	jr z,l1627h		;1613
	ld a,0a8h		;1615
	ld d,0dch		;1617
	out (010h),a		;1619
	halt			;161b
	outi			;161c
	jp nz,0151bh		;161e
	dec e			;1621
	jp nz,0151bh		;1622
	jr l1637h		;1625
l1627h:
	ld a,088h		;1627
	ld d,09ch		;1629
	out (010h),a		;162b
	halt			;162d
	ini			;162e
	jp nz,l152dh		;1630
	dec e			;1633
	jp nz,l152dh		;1634
l1637h:
	call 00fb5h		;1637
	and d			;163a
	pop bc			;163b
	call sub_117eh		;163c
	jr z,l1643h		;163f
	djnz l15f2h		;1641
l1643h:
	ld hl,0fdd7h		;1643
	or (hl)			;1646
	call 01560h		;1647
	ret z			;164a
	xor a			;164b
	inc a			;164c
	ld (hl),a		;164d
	ret			;164e
	ld a,e			;164f
sub_1650h:
	ld e,001h		;1650
	or a			;1652
	jr nz,l1658h		;1653
	ld b,080h		;1655
	ret			;1657
l1658h:
	dec a			;1658
	ret z			;1659
	inc e			;165a
	dec a			;165b
	ret z			;165c
	inc e			;165d
	inc e			;165e
	ret			;165f
	push af			;1660
	ld a,(0fdedh)		;1661
	bit 6,a			;1664
	jr z,l1683h		;1666
	push hl			;1668
	and 003h		;1669
	ld b,a			;166b
	inc b			;166c
	ld hl,00080h		;166d
	jr l1673h		;1670
l1672h:
	add hl,hl		;1672
l1673h:
	djnz l1672h		;1673
	ex de,hl		;1675
	ld hl,(0fff2h)		;1676
l1679h:
	ld a,(hl)		;1679
	cpl			;167a
	ld (hl),a		;167b
	inc hl			;167c
	dec de			;167d
	ld a,e			;167e
	or d			;167f
	jr nz,l1679h		;1680
	pop hl			;1682
l1683h:
	pop af			;1683
	ret			;1684
	ld b,005h		;1685
l1687h:
	ld a,(0fdc3h)		;1687
	ld hl,0fde2h		;168a
l168dh:
	sub (hl)		;168d
	ret z			;168e
	push bc			;168f
	ld c,060h		;1690
	jr nc,l1698h		;1692
	ld c,040h		;1694
	cpl			;1696
	inc a			;1697
l1698h:
	ld b,a			;1698
	ld hl,0fdech		;1699
	ld a,(hl)		;169c
	inc hl			;169d
	xor (hl)		;169e
	and 080h		;169f
	rla			;16a1
l16a2h:
	push bc			;16a2
	push af			;16a3
	call 015ech		;16a4
	pop af			;16a7
	pop bc			;16a8
	jr nc,l16b2h		;16a9
	push bc			;16ab
	push af			;16ac
	call 015ech		;16ad
	pop af			;16b0
	pop bc			;16b1
l16b2h:
	djnz l16a2h		;16b2
	ld a,(0fe77h)		;16b4
	call 01642h		;16b7
	call 0166eh		;16ba
	pop bc			;16bd
	jr nz,l16c2h		;16be
	djnz l1687h		;16c0
l16c2h:
	xor a			;16c2
	dec a			;16c3
	ret			;16c4
	ld c,040h		;16c5
	push bc			;16c7
	call 015ech		;16c8
	pop bc			;16cb
	call 015ech		;16cc
	call 015e2h		;16cf
	ld bc,00060h		;16d2
l16d5h:
	push bc			;16d5
	call 015ech		;16d6
	pop bc			;16d9
	and 004h		;16da
	jr nz,l16e2h		;16dc
	djnz l16d5h		;16de
	dec a			;16e0
	ret			;16e1
l16e2h:
	ld a,(0fe77h)		;16e2
	call 01642h		;16e5
	xor a			;16e8
	ret			;16e9
	ld c,040h		;16ea
	ld a,c			;16ec
	out (010h),a		;16ed
	ld a,(0fe75h)		;16ef
	call 01642h		;16f2
	call sub_1170h		;16f5
	call 00fb5h		;16f8
	call sub_117eh		;16fb
	ret			;16fe
	ld de,01611h		;16ff
	push de			;1702
	ld e,004h		;1703
	ld bc,00807h		;1705
	xor a			;1708
	ld h,a			;1709
	ld l,a			;170a
	out (005h),a		;170b
	out (005h),a		;170d
	jr l1756h		;170f
	xor a			;1711
	out (005h),a		;1712
	ld a,e			;1714
	ld de,01611h		;1715
	push de			;1718
	ld e,a			;1719
	djnz l1756h		;171a
	pop de			;171c
	ld a,h			;171d
	cpl			;171e
	ld h,a			;171f
	ld a,l			;1720
	cpl			;1721
	ld l,a			;1722
	inc hl			;1723
	ld bc,00000h		;1724
l1727h:
	ld de,0fef6h		;1727
	add hl,de		;172a
	inc bc			;172b
	bit 7,h			;172c
	jr z,l1727h		;172e
	ld (0fdc9h),bc		;1730
	ret			;1734
	or a			;1735
	ret z			;1736
	ld b,a			;1737
l1738h:
	push bc			;1738
	ld a,00ah		;1739
	call 01642h		;173b
	pop bc			;173e
	djnz l1738h		;173f
	ret			;1741
	or a			;1742
	ret z			;1743
	push af			;1744
	ld a,(0fe58h)		;1745
	inc a			;1748
	ld b,a			;1749
	pop af			;174a
	ld c,007h		;174b
	ld de,01663h		;174d
	push de			;1750
	ld e,b			;1751
	ld b,a			;1752
l1753h:
	ld hl,(0fdc9h)		;1753
l1756h:
	in a,(c)		;1756
	and e			;1758
	ret nz			;1759
l175ah:
	dec hl			;175a
	ld a,h			;175b
	or l			;175c
	jr nz,l1756h		;175d
	djnz l1753h		;175f
	pop de			;1761
	ret			;1762
	call l062ah		;1763
	ld a,e			;1766
	ld de,01663h		;1767
	push de			;176a
	ld e,a			;176b
	jr l175ah		;176c
	call sub_1170h		;176e
	call l0f23h		;1771
	ld a,0c0h		;1774
	out (010h),a		;1776
	ld de,l168dh		;1778
	ld bc,00210h		;177b
	ld a,0c8h		;177e
	call sub_1650h		;1780
	call 00fa7h		;1783
	xor a			;1786
	dec a			;1787
l1788h:
	or a			;1788
	call sub_117eh		;1789
	ret			;178c
	pop hl			;178d
	call 00fa7h		;178e
	in a,(013h)		;1791
	ld b,00ah		;1793
l1795h:
	push bc			;1795
	ld hl,0fdc3h		;1796
	ld bc,l0613h		;1799
	ld a,0c0h		;179c
	out (010h),a		;179e
	halt			;17a0
	ini			;17a1
	jp nz,016a0h		;17a3
	call 00fb5h		;17a6
	and 01ch		;17a9
	pop bc			;17ab
	jr z,l1788h		;17ac
	djnz l1795h		;17ae
	jr l1788h		;17b0
	nop			;17b2
	nop			;17b3
	ld a,(bc)		;17b4
	jr z,sub_17b7h		;17b5
sub_17b7h:
	inc bc			;17b7
	rlca			;17b8
	nop			;17b9
	jp nz,03f00h		;17ba
	nop			;17bd
	ret p			;17be
	nop			;17bf
	djnz l17c2h		;17c0
l17c2h:
	ld bc,00000h		;17c2
	nop			;17c5
	ld c,028h		;17c6
	nop			;17c8
	inc b			;17c9
	rrca			;17ca
	ld bc,000c4h		;17cb
	ccf			;17ce
	nop			;17cf
	ret nz			;17d0
	nop			;17d1
	djnz l17d4h		;17d2
l17d4h:
	ld bc,00000h		;17d4
	nop			;17d7
	sub d			;17d8
	jr z,l17dbh		;17d9
l17dbh:
	dec b			;17db
	rra			;17dc
	inc bc			;17dd
	call nz,05f00h		;17de
	nop			;17e1
	add a,b			;17e2
	nop			;17e3
	jr l17e6h		;17e4
l17e6h:
	ld (bc),a		;17e6
	nop			;17e7
	add a,d			;17e8
	rla			;17e9
	dec bc			;17ea
	jr z,l17edh		;17eb
l17edh:
	inc bc			;17ed
	rlca			;17ee
	nop			;17ef
	cp b			;17f0
	nop			;17f1
	ccf			;17f2
	nop			;17f3
	ret nz			;17f4
	nop			;17f5
	djnz l17f8h		;17f6
l17f8h:
	inc bc			;17f8
	nop			;17f9
	add a,d			;17fa
	rla			;17fb
	rrca			;17fc
	jr z,l17ffh		;17fd
l17ffh:
	inc b			;17ff
	rrca			;1800
	ld bc,000c2h		;1801
	rst 38h			;1804
	nop			;1805
	ret p			;1806
	nop			;1807
	ld b,b			;1808
	nop			;1809
	ld (bc),a		;180a
	nop			;180b
	add a,d			;180c
	rla			;180d
	sub e			;180e
	jr z,l1811h		;180f
l1811h:
	inc b			;1811
	rrca			;1812
	nop			;1813
	adc a,d			;1814
	ld bc,000ffh		;1815
	ret p			;1818
	nop			;1819
	ld b,b			;181a
	nop			;181b
	ld (bc),a		;181c
	nop			;181d
	ld h,(hl)		;181e
	rla			;181f
	jr z,l1834h		;1820
	nop			;1822
	inc bc			;1823
	rlca			;1824
l1825h:
	nop			;1825
	ld d,d			;1826
	nop			;1827
	rra			;1828
	nop			;1829
	add a,b			;182a
	nop			;182b
	ex af,af'		;182c
	nop			;182d
	inc bc			;182e
	nop			;182f
	ld a,b			;1830
	rla			;1831
	add hl,hl		;1832
	inc d			;1833
l1834h:
	nop			;1834
	inc b			;1835
	rrca			;1836
	ld bc,0002dh		;1837
	ccf			;183a
	nop			;183b
	add a,b			;183c
	nop			;183d
	djnz l1840h		;183e
l1840h:
	inc bc			;1840
	nop			;1841
	ld a,b			;1842
	rla			;1843
	ld a,(de)		;1844
	jr z,l1847h		;1845
l1847h:
	inc b			;1847
	rrca			;1848
	ld bc,000bdh		;1849
	ld a,a			;184c
	nop			;184d
	ret nz			;184e
	nop			;184f
	jr nz,l1852h		;1850
l1852h:
	inc b			;1852
	nop			;1853
	nop			;1854
	nop			;1855
	ld (bc),a		;1856
	ld b,h			;1857
	nop			;1858
	dec b			;1859
	rra			;185a
	ld bc,sub_0464h+1	;185b
	rst 38h			;185e
	inc bc			;185f
	rst 38h			;1860
	nop			;1861
	nop			;1862
	nop			;1863
	inc b			;1864
	nop			;1865
	ld bc,sub_0b06h		;1866
	djnz l186eh		;1869
	ex af,af'		;186b
	dec c			;186c
	ld (de),a		;186d
l186eh:
	dec b			;186e
	ld a,(bc)		;186f
	rrca			;1870
	ld (bc),a		;1871
	rlca			;1872
	inc c			;1873
	ld de,00904h		;1874
	ld c,001h		;1877
	inc bc			;1879
	dec b			;187a
	rlca			;187b
	add hl,bc		;187c
	ld (bc),a		;187d
	inc b			;187e
l187fh:
	ld b,008h		;187f
	ld a,(bc)		;1881
	ld bc,l0302h		;1882
	inc b			;1885
	dec b			;1886
	inc bc			;1887
	rlca			;1888
	nop			;1889
	rst 30h			;188a
	nop			;188b
	ccf			;188c
	inc b			;188d
	rrca			;188e
	ld bc,000fbh		;188f
	ld a,a			;1892
	inc b			;1893
	rrca			;1894
	nop			;1895
	ld a,e			;1896
	ld bc,l057fh		;1897
	rra			;189a
	inc bc			;189b
	defb 0fdh,000h,0ffh ;illegal sequence	;189c
	dec b			;189f
	rra			;18a0
	ld bc,0013dh		;18a1
	rst 38h			;18a4
	dec b			;18a5
	rra			;18a6
	ld bc,l017dh		;18a7
	rst 38h			;18aa
	dec b			;18ab
	rra			;18ac
l18adh:
	ld bc,l01bdh		;18ad
	rst 38h			;18b0
	dec b			;18b1
	rra			;18b2
	ld bc,l01fch+1		;18b3
	rst 38h			;18b6
	ld a,(0fdebh)		;18b7
	bit 2,a			;18ba
	ld c,003h		;18bc
	jr nz,l18c2h		;18be
	ld c,001h		;18c0
l18c2h:
	and c			;18c2
	ld (0fde4h),a		;18c3
	ld hl,l18adh		;18c6
	push hl			;18c9
	ld de,(0fde9h)		;18ca
	ld a,(0fde7h)		;18ce
	ld c,a			;18d1
	ld a,(0fdech)		;18d2
	ld b,a			;18d5
	ld a,(0fdedh)		;18d6
	rrca			;18d9
	rrca			;18da
	and 007h		;18db
	jp z,l187fh		;18dd
	dec a			;18e0
	jp z,l1825h		;18e1
	dec a			;18e4
	jr z,l18f6h		;18e5
	dec a			;18e7
	jr z,l18fah		;18e8
	dec a			;18ea
	jr z,l190bh		;18eb
	dec a			;18ed
	jr z,l190fh		;18ee
	dec a			;18f0
	jr z,l1921h		;18f1
	jp 0fdf3h		;18f3
l18f6h:
	ld b,000h		;18f6
l18f8h:
	ld l,e			;18f8
	ret			;18f9
l18fah:
	ld b,00ah		;18fa
l18fch:
	xor a			;18fc
	srl e			;18fd
	rla			;18ff
	or a			;1900
	jr z,l1908h		;1901
	push af			;1903
	ld a,c			;1904
	add a,b			;1905
	ld c,a			;1906
	pop af			;1907
l1908h:
	ld b,a			;1908
	jr l18f8h		;1909
l190bh:
	ld b,014h		;190b
	jr l18fch		;190d
l190fh:
	ld a,(0fdedh)		;190f
sub_1912h:
	and 003h		;1912
	inc a			;1914
	ld b,a			;1915
	ld a,(0ff55h)		;1916
	rla			;1919
l191ah:
	or a			;191a
sub_191bh:
	rra			;191b
	djnz l191ah		;191c
	ld b,a			;191e
	jr l18fch		;191f
l1921h:
	ld b,000h		;1921
	jr l18fch		;1923
	push bc			;1925
	call sub_1170h		;1926
	ld (0fe53h),sp		;1929
	ld a,(0fde4h)		;192d
	or a			;1930
	ld sp,(0fe59h)		;1931
	jr z,sub_193bh		;1935
	ld sp,(0fe5fh)		;1937
sub_193bh:
	pop hl			;193b
	sbc hl,de		;193c
	jr z,l1943h		;193e
	jr nc,l1946h		;1940
	ccf			;1942
l1943h:
	inc de			;1943
	jr sub_193bh		;1944
l1946h:
	ld sp,(0fe53h)		;1946
	call sub_117eh		;194a
	pop bc			;194d
	push bc			;194e
	ld a,b			;194f
	and 0e0h		;1950
	rlca			;1952
	rlca			;1953
	rlca			;1954
	inc a			;1955
	ld l,a			;1956
	ld b,0ffh		;1957
	ld a,l			;1959
	dec a			;195a
	cpl			;195b
	ld c,a			;195c
	ld hl,00000h		;195d
	ld a,d			;1960
	add a,c			;1961
	ld a,010h		;1962
	jr c,l196ah		;1964
	ld l,d			;1966
	ld d,e			;1967
	ld e,h			;1968
	rrca			;1969
l196ah:
	add hl,hl		;196a
	ex de,hl		;196b
	add hl,hl		;196c
	ex de,hl		;196d
	jr nc,l1971h		;196e
	inc hl			;1970
l1971h:
	push hl			;1971
	add hl,bc		;1972
	pop hl			;1973
	jr nc,l1978h		;1974
	add hl,bc		;1976
	inc de			;1977
l1978h:
	dec a			;1978
	jr nz,l196ah		;1979
	ex de,hl		;197b
	pop bc			;197c
	ld b,e			;197d
	ret			;197e
	ex de,hl		;197f
	push bc			;1980
	ld de,00004h		;1981
	ld a,l			;1984
	and 0f8h		;1985
	or h			;1987
sub_1988h:
	jr nz,l1994h		;1988
	ld e,a			;198a
l198bh:
	ld a,l			;198b
	cp 004h			;198c
	jr c,l1994h		;198e
	sub 004h		;1990
	add a,l			;1992
	ld l,a			;1993
l1994h:
	add hl,de		;1994
	srl h			;1995
	rr l			;1997
	ex de,hl		;1999
	ld a,000h		;199a
	rla			;199c
	ld c,a			;199d
	ex (sp),hl		;199e
	ld a,h			;199f
	rla			;19a0
	ld a,000h		;19a1
	jr nc,l19a7h		;19a3
	ld a,002h		;19a5
l19a7h:
	add a,c			;19a7
	pop bc			;19a8
	ld b,a			;19a9
	ld c,l			;19aa
	ex de,hl		;19ab
	ret			;19ac
	ld (0fde2h),hl		;19ad
	ld hl,0fddeh		;19b0
	ld (hl),c		;19b3
	inc hl			;19b4
	ld (hl),b		;19b5
	inc hl			;19b6
	ld (hl),e		;19b7
	inc hl			;19b8
	ld (hl),d		;19b9
	ret			;19ba
	call sub_191bh		;19bb
	jr z,l19fch		;19be
	ld hl,00000h		;19c0
	ld bc,08088h		;19c3
	otir			;19c6
	call sub_191bh		;19c8
	scf			;19cb
	ret nz			;19cc
	ld hl,0fed5h		;19cd
	ld (0fdd8h),hl		;19d0
	ld b,080h		;19d3
l19d5h:
	ld (hl),0e5h		;19d5
	inc hl			;19d7
	djnz l19d5h		;19d8
	ld a,001h		;19da
	ld (0fdceh),a		;19dc
	jr l19f6h		;19df
l19e1h:
	push af			;19e1
	call 012f2h		;19e2
	pop af			;19e5
l19e6h:
	inc a			;19e6
	cp 040h			;19e7
	jr nz,l19f7h		;19e9
	ld a,(0fdceh)		;19eb
	inc a			;19ee
	ld (0fdceh),a		;19ef
	sub 004h		;19f2
	jr z,l19fch		;19f4
l19f6h:
	xor a			;19f6
l19f7h:
	ld (0fdcch),a		;19f7
	jr l19e1h		;19fa
l19fch:
	xor a			;19fc
	call sub_1912h		;19fd
	cp 003h			;1a00
	ld c,a			;1a02
	jr nz,l1a0fh		;1a03
	ld a,080h		;1a05
	call sub_1912h		;1a07
	jr nz,l1a0fh		;1a0a
	set 2,a			;1a0c
	ld c,a			;1a0e
l1a0fh:
	ld a,c			;1a0f
	or a			;1a10
	ret			;1a11
	out (08ah),a		;1a12
	in a,(08bh)		;1a14
	and 007h		;1a16
	bit 2,a			;1a18
	ret			;1a1a
	xor a			;1a1b
	out (08ah),a		;1a1c
	ld a,03fh		;1a1e
	out (089h),a		;1a20
	ld hl,0fed5h		;1a22
	ld bc,07f88h		;1a25
	push hl			;1a28
	inir			;1a29
	ini			;1a2b
	pop hl			;1a2d
	ld b,080h		;1a2e
	ld de,00000h		;1a30
l1a33h:
	ld a,(de)		;1a33
	cp (hl)			;1a34
	ret nz			;1a35
	inc hl			;1a36
	inc de			;1a37
	djnz l1a33h		;1a38
	ret			;1a3a
	ld hl,l198bh		;1a3b
	ld de,0fe7ah		;1a3e
	ld bc,0005bh		;1a41
	ldir			;1a44
	ld hl,l19e6h		;1a46
	ld de,0fff8h		;1a49
	ld bc,00008h		;1a4c
	ldir			;1a4f
	xor a			;1a51
	ld (0fdc2h),a		;1a52
	ld (0ff98h),a		;1a55
	ld (0fe79h),a		;1a58
	ld h,a			;1a5b
	ld l,a			;1a5c
	ld (0ffe6h),hl		;1a5d
	ld (0ffe4h),hl		;1a60
	dec a			;1a63
	ld (0fe58h),a		;1a64
	ld a,004h		;1a67
	ld (0fe57h),a		;1a69
	ld hl,0ff9ch		;1a6c
	ld (0ffbeh),hl		;1a6f
	ld (0ffbch),hl		;1a72
	ld hl,0fdf3h		;1a75
	ld (0ffeeh),hl		;1a78
	ld hl,0ffc0h		;1a7b
	ld (0fff0h),hl		;1a7e
	ld hl,0fe59h		;1a81
	ld (0ffech),hl		;1a84
	ret			;1a87
	jp 0fecch		;1a88
	call 0fea1h		;1a8b
	jr z,l1a96h		;1a8e
	jr c,l1aadh		;1a90
	ldir			;1a92
	jr l1aa1h		;1a94
l1a96h:
	jr nc,l1a9fh		;1a96
	dec b			;1a98
	inir			;1a99
	ini			;1a9b
	jr l1aa1h		;1a9d
l1a9fh:
	otir			;1a9f
l1aa1h:
	push af			;1aa1
	in a,(014h)		;1aa2
	set 7,a			;1aa4
	call 0fecch		;1aa6
	out (014h),a		;1aa9
	pop af			;1aab
	ret			;1aac
l1aadh:
	call 0fdf0h		;1aad
	jr l1aa1h		;1ab0
	push af			;1ab2
	in a,(014h)		;1ab3
	res 7,a			;1ab5
	call 0fecch		;1ab7
	out (014h),a		;1aba
	pop af			;1abc
	ret			;1abd
	ld (0ff96h),sp		;1abe
	ld sp,0ff96h		;1ac2
	push af			;1ac5
	call 0fe90h		;1ac6
	call sub_061bh		;1ac9
	ld h,000h		;1acc
	pop af			;1ace
	push hl			;1acf
	ld hl,0fec4h		;1ad0
	ex (sp),hl		;1ad3
	jp (hl)			;1ad4
	call 0fea1h		;1ad5
	ld sp,(0ff96h)		;1ad8
	ret			;1adc
	and 0bfh		;1add
	push hl			;1adf
	ld hl,0fdc2h		;1ae0
	or (hl)			;1ae3
	pop hl			;1ae4
	ret			;1ae5
	ld d,b			;1ae6
	ld d,b			;1ae7
	ld d,e			;1ae8
	or h			;1ae9
	jp 0feadh		;1aea
	ex de,hl		;1aed
	rst 38h			;1aee
	rst 38h			;1aef
	rst 38h			;1af0
	rst 38h			;1af1
	rst 38h			;1af2
	rst 38h			;1af3
	rst 38h			;1af4
	rst 38h			;1af5
	rst 38h			;1af6
	rst 38h			;1af7
	rst 38h			;1af8
	rst 38h			;1af9
	rst 38h			;1afa
	rst 38h			;1afb
	rst 38h			;1afc
	rst 38h			;1afd
	rst 38h			;1afe
	rst 38h			;1aff
	rst 38h			;1b00
	rst 38h			;1b01
	rst 38h			;1b02
	rst 38h			;1b03
	rst 38h			;1b04
	rst 38h			;1b05
	rst 38h			;1b06
	rst 38h			;1b07
	rst 38h			;1b08
	rst 38h			;1b09
	rst 38h			;1b0a
	rst 38h			;1b0b
	rst 38h			;1b0c
	rst 38h			;1b0d
	rst 38h			;1b0e
	rst 38h			;1b0f
	rst 38h			;1b10
	rst 38h			;1b11
	rst 38h			;1b12
	rst 38h			;1b13
	rst 38h			;1b14
	rst 38h			;1b15
	rst 38h			;1b16
	rst 38h			;1b17
	rst 38h			;1b18
	rst 38h			;1b19
	rst 38h			;1b1a
	rst 38h			;1b1b
	rst 38h			;1b1c
	rst 38h			;1b1d
	rst 38h			;1b1e
	rst 38h			;1b1f
	rst 38h			;1b20
	rst 38h			;1b21
	rst 38h			;1b22
	rst 38h			;1b23
	rst 38h			;1b24
	rst 38h			;1b25
	rst 38h			;1b26
	rst 38h			;1b27
	rst 38h			;1b28
	rst 38h			;1b29
	rst 38h			;1b2a
	rst 38h			;1b2b
	rst 38h			;1b2c
	rst 38h			;1b2d
	rst 38h			;1b2e
	rst 38h			;1b2f
	rst 38h			;1b30
	rst 38h			;1b31
	rst 38h			;1b32
	rst 38h			;1b33
	rst 38h			;1b34
	rst 38h			;1b35
	rst 38h			;1b36
	rst 38h			;1b37
	rst 38h			;1b38
	rst 38h			;1b39
	rst 38h			;1b3a
	rst 38h			;1b3b
	rst 38h			;1b3c
	rst 38h			;1b3d
	rst 38h			;1b3e
	rst 38h			;1b3f
	rst 38h			;1b40
	rst 38h			;1b41
	rst 38h			;1b42
	rst 38h			;1b43
	rst 38h			;1b44
	rst 38h			;1b45
	rst 38h			;1b46
	rst 38h			;1b47
	rst 38h			;1b48
	rst 38h			;1b49
	rst 38h			;1b4a
	rst 38h			;1b4b
	rst 38h			;1b4c
	rst 38h			;1b4d
	rst 38h			;1b4e
	rst 38h			;1b4f
	rst 38h			;1b50
	rst 38h			;1b51
	rst 38h			;1b52
	rst 38h			;1b53
	rst 38h			;1b54
	rst 38h			;1b55
	rst 38h			;1b56
	rst 38h			;1b57
	rst 38h			;1b58
	rst 38h			;1b59
	rst 38h			;1b5a
	rst 38h			;1b5b
	rst 38h			;1b5c
	rst 38h			;1b5d
	rst 38h			;1b5e
	rst 38h			;1b5f
	rst 38h			;1b60
	rst 38h			;1b61
	rst 38h			;1b62
	rst 38h			;1b63
	rst 38h			;1b64
	rst 38h			;1b65
	rst 38h			;1b66
	rst 38h			;1b67
	rst 38h			;1b68
	rst 38h			;1b69
	rst 38h			;1b6a
	rst 38h			;1b6b
	rst 38h			;1b6c
	rst 38h			;1b6d
	rst 38h			;1b6e
	rst 38h			;1b6f
	rst 38h			;1b70
	rst 38h			;1b71
	rst 38h			;1b72
	rst 38h			;1b73
	rst 38h			;1b74
	rst 38h			;1b75
	rst 38h			;1b76
	rst 38h			;1b77
	rst 38h			;1b78
	rst 38h			;1b79
	rst 38h			;1b7a
	rst 38h			;1b7b
	rst 38h			;1b7c
	rst 38h			;1b7d
	rst 38h			;1b7e
	rst 38h			;1b7f
	rst 38h			;1b80
	rst 38h			;1b81
	rst 38h			;1b82
	rst 38h			;1b83
	rst 38h			;1b84
	rst 38h			;1b85
	rst 38h			;1b86
	rst 38h			;1b87
	rst 38h			;1b88
	rst 38h			;1b89
	rst 38h			;1b8a
	rst 38h			;1b8b
	rst 38h			;1b8c
	rst 38h			;1b8d
	rst 38h			;1b8e
	rst 38h			;1b8f
	rst 38h			;1b90
	rst 38h			;1b91
	rst 38h			;1b92
	rst 38h			;1b93
	rst 38h			;1b94
	rst 38h			;1b95
	rst 38h			;1b96
	rst 38h			;1b97
	rst 38h			;1b98
	rst 38h			;1b99
	rst 38h			;1b9a
	rst 38h			;1b9b
	rst 38h			;1b9c
	rst 38h			;1b9d
	rst 38h			;1b9e
	rst 38h			;1b9f
	rst 38h			;1ba0
	rst 38h			;1ba1
	rst 38h			;1ba2
	rst 38h			;1ba3
	rst 38h			;1ba4
	rst 38h			;1ba5
	rst 38h			;1ba6
	rst 38h			;1ba7
	rst 38h			;1ba8
	rst 38h			;1ba9
	rst 38h			;1baa
	rst 38h			;1bab
	rst 38h			;1bac
	rst 38h			;1bad
	rst 38h			;1bae
	rst 38h			;1baf
	rst 38h			;1bb0
	rst 38h			;1bb1
	rst 38h			;1bb2
	rst 38h			;1bb3
	rst 38h			;1bb4
	rst 38h			;1bb5
	rst 38h			;1bb6
	rst 38h			;1bb7
	rst 38h			;1bb8
	rst 38h			;1bb9
	rst 38h			;1bba
	rst 38h			;1bbb
	rst 38h			;1bbc
	rst 38h			;1bbd
	rst 38h			;1bbe
	rst 38h			;1bbf
	rst 38h			;1bc0
	rst 38h			;1bc1
	rst 38h			;1bc2
	rst 38h			;1bc3
	rst 38h			;1bc4
	rst 38h			;1bc5
	rst 38h			;1bc6
	rst 38h			;1bc7
	rst 38h			;1bc8
	rst 38h			;1bc9
	rst 38h			;1bca
	rst 38h			;1bcb
	rst 38h			;1bcc
	rst 38h			;1bcd
	rst 38h			;1bce
	rst 38h			;1bcf
	rst 38h			;1bd0
	rst 38h			;1bd1
	rst 38h			;1bd2
	rst 38h			;1bd3
	rst 38h			;1bd4
	rst 38h			;1bd5
	rst 38h			;1bd6
	rst 38h			;1bd7
	rst 38h			;1bd8
	rst 38h			;1bd9
	rst 38h			;1bda
	rst 38h			;1bdb
	rst 38h			;1bdc
	rst 38h			;1bdd
	rst 38h			;1bde
	rst 38h			;1bdf
	rst 38h			;1be0
	rst 38h			;1be1
	rst 38h			;1be2
	rst 38h			;1be3
	rst 38h			;1be4
	rst 38h			;1be5
	rst 38h			;1be6
	rst 38h			;1be7
	rst 38h			;1be8
	rst 38h			;1be9
	rst 38h			;1bea
	rst 38h			;1beb
	rst 38h			;1bec
	rst 38h			;1bed
	rst 38h			;1bee
	rst 38h			;1bef
	rst 38h			;1bf0
	rst 38h			;1bf1
	rst 38h			;1bf2
	rst 38h			;1bf3
	rst 38h			;1bf4
	rst 38h			;1bf5
	rst 38h			;1bf6
	rst 38h			;1bf7
	rst 38h			;1bf8
	rst 38h			;1bf9
	rst 38h			;1bfa
	rst 38h			;1bfb
	rst 38h			;1bfc
	rst 38h			;1bfd
	rst 38h			;1bfe
	rst 38h			;1bff
	rst 38h			;1c00
	rst 38h			;1c01
	rst 38h			;1c02
	rst 38h			;1c03
	rst 38h			;1c04
	rst 38h			;1c05
	rst 38h			;1c06
	rst 38h			;1c07
	rst 38h			;1c08
	rst 38h			;1c09
	rst 38h			;1c0a
	rst 38h			;1c0b
	rst 38h			;1c0c
	rst 38h			;1c0d
	rst 38h			;1c0e
	rst 38h			;1c0f
	rst 38h			;1c10
	rst 38h			;1c11
	rst 38h			;1c12
	rst 38h			;1c13
	rst 38h			;1c14
	rst 38h			;1c15
	rst 38h			;1c16
	rst 38h			;1c17
	rst 38h			;1c18
	rst 38h			;1c19
	rst 38h			;1c1a
	rst 38h			;1c1b
	rst 38h			;1c1c
	rst 38h			;1c1d
	rst 38h			;1c1e
	rst 38h			;1c1f
	rst 38h			;1c20
	rst 38h			;1c21
	rst 38h			;1c22
	rst 38h			;1c23
	rst 38h			;1c24
	rst 38h			;1c25
	rst 38h			;1c26
	rst 38h			;1c27
	rst 38h			;1c28
	rst 38h			;1c29
	rst 38h			;1c2a
	rst 38h			;1c2b
	rst 38h			;1c2c
	rst 38h			;1c2d
	rst 38h			;1c2e
	rst 38h			;1c2f
	rst 38h			;1c30
	rst 38h			;1c31
	rst 38h			;1c32
	rst 38h			;1c33
	rst 38h			;1c34
	rst 38h			;1c35
	rst 38h			;1c36
	rst 38h			;1c37
	rst 38h			;1c38
	rst 38h			;1c39
	rst 38h			;1c3a
	rst 38h			;1c3b
	rst 38h			;1c3c
	rst 38h			;1c3d
	rst 38h			;1c3e
	rst 38h			;1c3f
	rst 38h			;1c40
	rst 38h			;1c41
	rst 38h			;1c42
	rst 38h			;1c43
	rst 38h			;1c44
	rst 38h			;1c45
	rst 38h			;1c46
	rst 38h			;1c47
	rst 38h			;1c48
	rst 38h			;1c49
	rst 38h			;1c4a
	rst 38h			;1c4b
	rst 38h			;1c4c
	rst 38h			;1c4d
	rst 38h			;1c4e
	rst 38h			;1c4f
	rst 38h			;1c50
	rst 38h			;1c51
	rst 38h			;1c52
	rst 38h			;1c53
	rst 38h			;1c54
	rst 38h			;1c55
	rst 38h			;1c56
	rst 38h			;1c57
	rst 38h			;1c58
	rst 38h			;1c59
	rst 38h			;1c5a
	rst 38h			;1c5b
	rst 38h			;1c5c
	rst 38h			;1c5d
	rst 38h			;1c5e
	rst 38h			;1c5f
	rst 38h			;1c60
	rst 38h			;1c61
	rst 38h			;1c62
	rst 38h			;1c63
	rst 38h			;1c64
	rst 38h			;1c65
	rst 38h			;1c66
	rst 38h			;1c67
	rst 38h			;1c68
	rst 38h			;1c69
	rst 38h			;1c6a
	rst 38h			;1c6b
	rst 38h			;1c6c
	rst 38h			;1c6d
	rst 38h			;1c6e
	rst 38h			;1c6f
	rst 38h			;1c70
	rst 38h			;1c71
	rst 38h			;1c72
	rst 38h			;1c73
	rst 38h			;1c74
	rst 38h			;1c75
	rst 38h			;1c76
	rst 38h			;1c77
	rst 38h			;1c78
	rst 38h			;1c79
	rst 38h			;1c7a
	rst 38h			;1c7b
	rst 38h			;1c7c
	rst 38h			;1c7d
	rst 38h			;1c7e
	rst 38h			;1c7f
	rst 38h			;1c80
	rst 38h			;1c81
	rst 38h			;1c82
	rst 38h			;1c83
	rst 38h			;1c84
	rst 38h			;1c85
	rst 38h			;1c86
	rst 38h			;1c87
	rst 38h			;1c88
	rst 38h			;1c89
	rst 38h			;1c8a
	rst 38h			;1c8b
	rst 38h			;1c8c
	rst 38h			;1c8d
	rst 38h			;1c8e
	rst 38h			;1c8f
	rst 38h			;1c90
	rst 38h			;1c91
	rst 38h			;1c92
	rst 38h			;1c93
	rst 38h			;1c94
	rst 38h			;1c95
	rst 38h			;1c96
	rst 38h			;1c97
	rst 38h			;1c98
	rst 38h			;1c99
	rst 38h			;1c9a
	rst 38h			;1c9b
	rst 38h			;1c9c
	rst 38h			;1c9d
	rst 38h			;1c9e
	rst 38h			;1c9f
	rst 38h			;1ca0
	rst 38h			;1ca1
	rst 38h			;1ca2
	rst 38h			;1ca3
	rst 38h			;1ca4
	rst 38h			;1ca5
	rst 38h			;1ca6
	rst 38h			;1ca7
	rst 38h			;1ca8
	rst 38h			;1ca9
	rst 38h			;1caa
	rst 38h			;1cab
	rst 38h			;1cac
	rst 38h			;1cad
	rst 38h			;1cae
	rst 38h			;1caf
	rst 38h			;1cb0
	rst 38h			;1cb1
	rst 38h			;1cb2
	rst 38h			;1cb3
	rst 38h			;1cb4
	rst 38h			;1cb5
	rst 38h			;1cb6
	rst 38h			;1cb7
	rst 38h			;1cb8
	rst 38h			;1cb9
	rst 38h			;1cba
	rst 38h			;1cbb
	rst 38h			;1cbc
	rst 38h			;1cbd
	rst 38h			;1cbe
	rst 38h			;1cbf
	rst 38h			;1cc0
	rst 38h			;1cc1
	rst 38h			;1cc2
	rst 38h			;1cc3
	rst 38h			;1cc4
	rst 38h			;1cc5
	rst 38h			;1cc6
	rst 38h			;1cc7
	rst 38h			;1cc8
	rst 38h			;1cc9
	rst 38h			;1cca
	rst 38h			;1ccb
	rst 38h			;1ccc
	rst 38h			;1ccd
	rst 38h			;1cce
	rst 38h			;1ccf
	rst 38h			;1cd0
	rst 38h			;1cd1
	rst 38h			;1cd2
	rst 38h			;1cd3
	rst 38h			;1cd4
	rst 38h			;1cd5
	rst 38h			;1cd6
	rst 38h			;1cd7
	rst 38h			;1cd8
	rst 38h			;1cd9
	rst 38h			;1cda
	rst 38h			;1cdb
	rst 38h			;1cdc
	rst 38h			;1cdd
	rst 38h			;1cde
	rst 38h			;1cdf
	rst 38h			;1ce0
	rst 38h			;1ce1
	rst 38h			;1ce2
	rst 38h			;1ce3
	rst 38h			;1ce4
	rst 38h			;1ce5
	rst 38h			;1ce6
	rst 38h			;1ce7
	rst 38h			;1ce8
	rst 38h			;1ce9
	rst 38h			;1cea
	rst 38h			;1ceb
	rst 38h			;1cec
	rst 38h			;1ced
	rst 38h			;1cee
	rst 38h			;1cef
	rst 38h			;1cf0
	rst 38h			;1cf1
	rst 38h			;1cf2
	rst 38h			;1cf3
	rst 38h			;1cf4
	rst 38h			;1cf5
	rst 38h			;1cf6
	rst 38h			;1cf7
	rst 38h			;1cf8
	rst 38h			;1cf9
	rst 38h			;1cfa
	rst 38h			;1cfb
	rst 38h			;1cfc
	rst 38h			;1cfd
	rst 38h			;1cfe
	rst 38h			;1cff
	rst 38h			;1d00
	rst 38h			;1d01
	rst 38h			;1d02
	rst 38h			;1d03
	rst 38h			;1d04
	rst 38h			;1d05
	rst 38h			;1d06
	rst 38h			;1d07
	rst 38h			;1d08
	rst 38h			;1d09
	rst 38h			;1d0a
	rst 38h			;1d0b
	rst 38h			;1d0c
	rst 38h			;1d0d
	rst 38h			;1d0e
	rst 38h			;1d0f
	rst 38h			;1d10
	rst 38h			;1d11
	rst 38h			;1d12
	rst 38h			;1d13
	rst 38h			;1d14
	rst 38h			;1d15
	rst 38h			;1d16
	rst 38h			;1d17
	rst 38h			;1d18
	rst 38h			;1d19
	rst 38h			;1d1a
	rst 38h			;1d1b
	rst 38h			;1d1c
	rst 38h			;1d1d
	rst 38h			;1d1e
	rst 38h			;1d1f
	rst 38h			;1d20
	rst 38h			;1d21
	rst 38h			;1d22
	rst 38h			;1d23
	rst 38h			;1d24
	rst 38h			;1d25
	rst 38h			;1d26
	rst 38h			;1d27
	rst 38h			;1d28
	rst 38h			;1d29
	rst 38h			;1d2a
	rst 38h			;1d2b
	rst 38h			;1d2c
	rst 38h			;1d2d
	rst 38h			;1d2e
	rst 38h			;1d2f
	rst 38h			;1d30
	rst 38h			;1d31
	rst 38h			;1d32
	rst 38h			;1d33
	rst 38h			;1d34
	rst 38h			;1d35
	rst 38h			;1d36
	rst 38h			;1d37
	rst 38h			;1d38
	rst 38h			;1d39
	rst 38h			;1d3a
	rst 38h			;1d3b
	rst 38h			;1d3c
	rst 38h			;1d3d
	rst 38h			;1d3e
	rst 38h			;1d3f
	rst 38h			;1d40
	rst 38h			;1d41
	rst 38h			;1d42
	rst 38h			;1d43
	rst 38h			;1d44
	rst 38h			;1d45
	rst 38h			;1d46
	rst 38h			;1d47
	rst 38h			;1d48
	rst 38h			;1d49
	rst 38h			;1d4a
	rst 38h			;1d4b
	rst 38h			;1d4c
	rst 38h			;1d4d
	rst 38h			;1d4e
	rst 38h			;1d4f
	rst 38h			;1d50
	rst 38h			;1d51
	rst 38h			;1d52
	rst 38h			;1d53
	rst 38h			;1d54
	rst 38h			;1d55
	rst 38h			;1d56
	rst 38h			;1d57
	rst 38h			;1d58
	rst 38h			;1d59
	rst 38h			;1d5a
	rst 38h			;1d5b
	rst 38h			;1d5c
	rst 38h			;1d5d
	rst 38h			;1d5e
	rst 38h			;1d5f
	rst 38h			;1d60
	rst 38h			;1d61
	rst 38h			;1d62
	rst 38h			;1d63
	rst 38h			;1d64
	rst 38h			;1d65
	rst 38h			;1d66
	rst 38h			;1d67
	rst 38h			;1d68
	rst 38h			;1d69
	rst 38h			;1d6a
	rst 38h			;1d6b
	rst 38h			;1d6c
	rst 38h			;1d6d
	rst 38h			;1d6e
	rst 38h			;1d6f
	rst 38h			;1d70
	rst 38h			;1d71
	rst 38h			;1d72
	rst 38h			;1d73
	rst 38h			;1d74
	rst 38h			;1d75
	rst 38h			;1d76
	rst 38h			;1d77
	rst 38h			;1d78
	rst 38h			;1d79
	rst 38h			;1d7a
	rst 38h			;1d7b
	rst 38h			;1d7c
	rst 38h			;1d7d
	rst 38h			;1d7e
	rst 38h			;1d7f
	rst 38h			;1d80
	rst 38h			;1d81
	rst 38h			;1d82
	rst 38h			;1d83
	rst 38h			;1d84
	rst 38h			;1d85
	rst 38h			;1d86
	rst 38h			;1d87
	rst 38h			;1d88
	rst 38h			;1d89
	rst 38h			;1d8a
	rst 38h			;1d8b
	rst 38h			;1d8c
	rst 38h			;1d8d
	rst 38h			;1d8e
	rst 38h			;1d8f
	rst 38h			;1d90
	rst 38h			;1d91
	rst 38h			;1d92
	rst 38h			;1d93
	rst 38h			;1d94
	rst 38h			;1d95
	rst 38h			;1d96
	rst 38h			;1d97
	rst 38h			;1d98
	rst 38h			;1d99
	rst 38h			;1d9a
	rst 38h			;1d9b
	rst 38h			;1d9c
	rst 38h			;1d9d
	rst 38h			;1d9e
	rst 38h			;1d9f
	rst 38h			;1da0
	rst 38h			;1da1
	rst 38h			;1da2
	rst 38h			;1da3
	rst 38h			;1da4
	rst 38h			;1da5
	rst 38h			;1da6
	rst 38h			;1da7
	rst 38h			;1da8
	rst 38h			;1da9
	rst 38h			;1daa
	rst 38h			;1dab
	rst 38h			;1dac
	rst 38h			;1dad
	rst 38h			;1dae
	rst 38h			;1daf
	rst 38h			;1db0
	rst 38h			;1db1
	rst 38h			;1db2
	rst 38h			;1db3
	rst 38h			;1db4
	rst 38h			;1db5
	rst 38h			;1db6
	rst 38h			;1db7
	rst 38h			;1db8
	rst 38h			;1db9
	rst 38h			;1dba
	rst 38h			;1dbb
	rst 38h			;1dbc
	rst 38h			;1dbd
	rst 38h			;1dbe
	rst 38h			;1dbf
	rst 38h			;1dc0
	rst 38h			;1dc1
	rst 38h			;1dc2
	rst 38h			;1dc3
	rst 38h			;1dc4
	rst 38h			;1dc5
	rst 38h			;1dc6
	rst 38h			;1dc7
	rst 38h			;1dc8
	rst 38h			;1dc9
	rst 38h			;1dca
	rst 38h			;1dcb
	rst 38h			;1dcc
	rst 38h			;1dcd
	rst 38h			;1dce
	rst 38h			;1dcf
	rst 38h			;1dd0
	rst 38h			;1dd1
	rst 38h			;1dd2
	rst 38h			;1dd3
	rst 38h			;1dd4
	rst 38h			;1dd5
	rst 38h			;1dd6
	rst 38h			;1dd7
	rst 38h			;1dd8
	rst 38h			;1dd9
	rst 38h			;1dda
	rst 38h			;1ddb
	rst 38h			;1ddc
	rst 38h			;1ddd
	rst 38h			;1dde
	rst 38h			;1ddf
	rst 38h			;1de0
	rst 38h			;1de1
	rst 38h			;1de2
	rst 38h			;1de3
	rst 38h			;1de4
	rst 38h			;1de5
	rst 38h			;1de6
	rst 38h			;1de7
	rst 38h			;1de8
	rst 38h			;1de9
	rst 38h			;1dea
	rst 38h			;1deb
	rst 38h			;1dec
	rst 38h			;1ded
	rst 38h			;1dee
	rst 38h			;1def
	rst 38h			;1df0
	rst 38h			;1df1
	rst 38h			;1df2
	rst 38h			;1df3
	rst 38h			;1df4
	rst 38h			;1df5
	rst 38h			;1df6
	rst 38h			;1df7
	rst 38h			;1df8
	rst 38h			;1df9
	rst 38h			;1dfa
	rst 38h			;1dfb
	rst 38h			;1dfc
	rst 38h			;1dfd
	rst 38h			;1dfe
	rst 38h			;1dff
	rst 38h			;1e00
	rst 38h			;1e01
	rst 38h			;1e02
	rst 38h			;1e03
	rst 38h			;1e04
	rst 38h			;1e05
	rst 38h			;1e06
	rst 38h			;1e07
	rst 38h			;1e08
	rst 38h			;1e09
	rst 38h			;1e0a
	rst 38h			;1e0b
	rst 38h			;1e0c
	rst 38h			;1e0d
	rst 38h			;1e0e
	rst 38h			;1e0f
	rst 38h			;1e10
	rst 38h			;1e11
	rst 38h			;1e12
	rst 38h			;1e13
	rst 38h			;1e14
	rst 38h			;1e15
	rst 38h			;1e16
	rst 38h			;1e17
	rst 38h			;1e18
	rst 38h			;1e19
	rst 38h			;1e1a
	rst 38h			;1e1b
	rst 38h			;1e1c
	rst 38h			;1e1d
	rst 38h			;1e1e
	rst 38h			;1e1f
	rst 38h			;1e20
	rst 38h			;1e21
	rst 38h			;1e22
	rst 38h			;1e23
	rst 38h			;1e24
	rst 38h			;1e25
	rst 38h			;1e26
	rst 38h			;1e27
	rst 38h			;1e28
	rst 38h			;1e29
	rst 38h			;1e2a
	rst 38h			;1e2b
	rst 38h			;1e2c
	rst 38h			;1e2d
	rst 38h			;1e2e
	rst 38h			;1e2f
	rst 38h			;1e30
	rst 38h			;1e31
	rst 38h			;1e32
	rst 38h			;1e33
	rst 38h			;1e34
	rst 38h			;1e35
	rst 38h			;1e36
	rst 38h			;1e37
	rst 38h			;1e38
	rst 38h			;1e39
	rst 38h			;1e3a
	rst 38h			;1e3b
	rst 38h			;1e3c
	rst 38h			;1e3d
	rst 38h			;1e3e
	rst 38h			;1e3f
	rst 38h			;1e40
	rst 38h			;1e41
	rst 38h			;1e42
	rst 38h			;1e43
	rst 38h			;1e44
	rst 38h			;1e45
	rst 38h			;1e46
	rst 38h			;1e47
	rst 38h			;1e48
	rst 38h			;1e49
	rst 38h			;1e4a
	rst 38h			;1e4b
	rst 38h			;1e4c
	rst 38h			;1e4d
	rst 38h			;1e4e
	rst 38h			;1e4f
	rst 38h			;1e50
	rst 38h			;1e51
	rst 38h			;1e52
	rst 38h			;1e53
	rst 38h			;1e54
	rst 38h			;1e55
	rst 38h			;1e56
	rst 38h			;1e57
	rst 38h			;1e58
	rst 38h			;1e59
	rst 38h			;1e5a
	rst 38h			;1e5b
	rst 38h			;1e5c
	rst 38h			;1e5d
	rst 38h			;1e5e
	rst 38h			;1e5f
	rst 38h			;1e60
	rst 38h			;1e61
	rst 38h			;1e62
	rst 38h			;1e63
	rst 38h			;1e64
	rst 38h			;1e65
	rst 38h			;1e66
	rst 38h			;1e67
	rst 38h			;1e68
	rst 38h			;1e69
	rst 38h			;1e6a
	rst 38h			;1e6b
	rst 38h			;1e6c
	rst 38h			;1e6d
	rst 38h			;1e6e
	rst 38h			;1e6f
	rst 38h			;1e70
	rst 38h			;1e71
	rst 38h			;1e72
	rst 38h			;1e73
	rst 38h			;1e74
	rst 38h			;1e75
	rst 38h			;1e76
	rst 38h			;1e77
	rst 38h			;1e78
	rst 38h			;1e79
	rst 38h			;1e7a
	rst 38h			;1e7b
	rst 38h			;1e7c
	rst 38h			;1e7d
	rst 38h			;1e7e
	rst 38h			;1e7f
	rst 38h			;1e80
	rst 38h			;1e81
	rst 38h			;1e82
	rst 38h			;1e83
	rst 38h			;1e84
	rst 38h			;1e85
	rst 38h			;1e86
	rst 38h			;1e87
	rst 38h			;1e88
	rst 38h			;1e89
	rst 38h			;1e8a
	rst 38h			;1e8b
	rst 38h			;1e8c
	rst 38h			;1e8d
	rst 38h			;1e8e
	rst 38h			;1e8f
	rst 38h			;1e90
	rst 38h			;1e91
	rst 38h			;1e92
	rst 38h			;1e93
	rst 38h			;1e94
	rst 38h			;1e95
	rst 38h			;1e96
	rst 38h			;1e97
	rst 38h			;1e98
	rst 38h			;1e99
	rst 38h			;1e9a
	rst 38h			;1e9b
	rst 38h			;1e9c
	rst 38h			;1e9d
	rst 38h			;1e9e
	rst 38h			;1e9f
	rst 38h			;1ea0
	rst 38h			;1ea1
	rst 38h			;1ea2
	rst 38h			;1ea3
	rst 38h			;1ea4
	rst 38h			;1ea5
	rst 38h			;1ea6
	rst 38h			;1ea7
	rst 38h			;1ea8
	rst 38h			;1ea9
	rst 38h			;1eaa
	rst 38h			;1eab
	rst 38h			;1eac
	rst 38h			;1ead
	rst 38h			;1eae
	rst 38h			;1eaf
	rst 38h			;1eb0
	rst 38h			;1eb1
	rst 38h			;1eb2
	rst 38h			;1eb3
	rst 38h			;1eb4
	rst 38h			;1eb5
	rst 38h			;1eb6
	rst 38h			;1eb7
	rst 38h			;1eb8
	rst 38h			;1eb9
	rst 38h			;1eba
	rst 38h			;1ebb
	rst 38h			;1ebc
	rst 38h			;1ebd
	rst 38h			;1ebe
	rst 38h			;1ebf
	rst 38h			;1ec0
	rst 38h			;1ec1
	rst 38h			;1ec2
	rst 38h			;1ec3
	rst 38h			;1ec4
	rst 38h			;1ec5
	rst 38h			;1ec6
	rst 38h			;1ec7
	rst 38h			;1ec8
	rst 38h			;1ec9
	rst 38h			;1eca
	rst 38h			;1ecb
	rst 38h			;1ecc
	rst 38h			;1ecd
	rst 38h			;1ece
	rst 38h			;1ecf
	rst 38h			;1ed0
	rst 38h			;1ed1
	rst 38h			;1ed2
	rst 38h			;1ed3
	rst 38h			;1ed4
	rst 38h			;1ed5
	rst 38h			;1ed6
	rst 38h			;1ed7
	rst 38h			;1ed8
	rst 38h			;1ed9
	rst 38h			;1eda
	rst 38h			;1edb
	rst 38h			;1edc
	rst 38h			;1edd
	rst 38h			;1ede
	rst 38h			;1edf
	rst 38h			;1ee0
	rst 38h			;1ee1
	rst 38h			;1ee2
	rst 38h			;1ee3
	rst 38h			;1ee4
	rst 38h			;1ee5
	rst 38h			;1ee6
	rst 38h			;1ee7
	rst 38h			;1ee8
	rst 38h			;1ee9
	rst 38h			;1eea
	rst 38h			;1eeb
	rst 38h			;1eec
	rst 38h			;1eed
	rst 38h			;1eee
	rst 38h			;1eef
	rst 38h			;1ef0
	rst 38h			;1ef1
	rst 38h			;1ef2
	rst 38h			;1ef3
	rst 38h			;1ef4
	rst 38h			;1ef5
	rst 38h			;1ef6
	rst 38h			;1ef7
	rst 38h			;1ef8
	rst 38h			;1ef9
	rst 38h			;1efa
	rst 38h			;1efb
	rst 38h			;1efc
	rst 38h			;1efd
	rst 38h			;1efe
	rst 38h			;1eff
	rst 38h			;1f00
	rst 38h			;1f01
	rst 38h			;1f02
	rst 38h			;1f03
	rst 38h			;1f04
	rst 38h			;1f05
	rst 38h			;1f06
	rst 38h			;1f07
	rst 38h			;1f08
	rst 38h			;1f09
	rst 38h			;1f0a
	rst 38h			;1f0b
	rst 38h			;1f0c
	rst 38h			;1f0d
	rst 38h			;1f0e
	rst 38h			;1f0f
	rst 38h			;1f10
	rst 38h			;1f11
	rst 38h			;1f12
	rst 38h			;1f13
	rst 38h			;1f14
	rst 38h			;1f15
	rst 38h			;1f16
	rst 38h			;1f17
	rst 38h			;1f18
	rst 38h			;1f19
	rst 38h			;1f1a
	rst 38h			;1f1b
	rst 38h			;1f1c
	rst 38h			;1f1d
	rst 38h			;1f1e
	rst 38h			;1f1f
	rst 38h			;1f20
	rst 38h			;1f21
	rst 38h			;1f22
	rst 38h			;1f23
	rst 38h			;1f24
	rst 38h			;1f25
	rst 38h			;1f26
	rst 38h			;1f27
	rst 38h			;1f28
	rst 38h			;1f29
	rst 38h			;1f2a
	rst 38h			;1f2b
	rst 38h			;1f2c
	rst 38h			;1f2d
	rst 38h			;1f2e
	rst 38h			;1f2f
	rst 38h			;1f30
	rst 38h			;1f31
	rst 38h			;1f32
	rst 38h			;1f33
	rst 38h			;1f34
	rst 38h			;1f35
	rst 38h			;1f36
	rst 38h			;1f37
	rst 38h			;1f38
	rst 38h			;1f39
	rst 38h			;1f3a
	rst 38h			;1f3b
	rst 38h			;1f3c
	rst 38h			;1f3d
	rst 38h			;1f3e
	rst 38h			;1f3f
	rst 38h			;1f40
	rst 38h			;1f41
	rst 38h			;1f42
	rst 38h			;1f43
	rst 38h			;1f44
	rst 38h			;1f45
	rst 38h			;1f46
	rst 38h			;1f47
	rst 38h			;1f48
	rst 38h			;1f49
	rst 38h			;1f4a
	rst 38h			;1f4b
	rst 38h			;1f4c
	rst 38h			;1f4d
	rst 38h			;1f4e
	rst 38h			;1f4f
	rst 38h			;1f50
	rst 38h			;1f51
	rst 38h			;1f52
	rst 38h			;1f53
	rst 38h			;1f54
	rst 38h			;1f55
	rst 38h			;1f56
	rst 38h			;1f57
	rst 38h			;1f58
	rst 38h			;1f59
	rst 38h			;1f5a
	rst 38h			;1f5b
	rst 38h			;1f5c
	rst 38h			;1f5d
	rst 38h			;1f5e
	rst 38h			;1f5f
	rst 38h			;1f60
	rst 38h			;1f61
	rst 38h			;1f62
	rst 38h			;1f63
	rst 38h			;1f64
	rst 38h			;1f65
	rst 38h			;1f66
	rst 38h			;1f67
	rst 38h			;1f68
	rst 38h			;1f69
	rst 38h			;1f6a
	rst 38h			;1f6b
	rst 38h			;1f6c
	rst 38h			;1f6d
	rst 38h			;1f6e
	rst 38h			;1f6f
	rst 38h			;1f70
	rst 38h			;1f71
	rst 38h			;1f72
	rst 38h			;1f73
	rst 38h			;1f74
	rst 38h			;1f75
	rst 38h			;1f76
	rst 38h			;1f77
	rst 38h			;1f78
	rst 38h			;1f79
	rst 38h			;1f7a
	rst 38h			;1f7b
	rst 38h			;1f7c
	rst 38h			;1f7d
	rst 38h			;1f7e
	rst 38h			;1f7f
	rst 38h			;1f80
	rst 38h			;1f81
	rst 38h			;1f82
	rst 38h			;1f83
	rst 38h			;1f84
	rst 38h			;1f85
	rst 38h			;1f86
	rst 38h			;1f87
	rst 38h			;1f88
	rst 38h			;1f89
	rst 38h			;1f8a
	rst 38h			;1f8b
	rst 38h			;1f8c
	rst 38h			;1f8d
	rst 38h			;1f8e
	rst 38h			;1f8f
	rst 38h			;1f90
	rst 38h			;1f91
	rst 38h			;1f92
	rst 38h			;1f93
	rst 38h			;1f94
	rst 38h			;1f95
	rst 38h			;1f96
	rst 38h			;1f97
	rst 38h			;1f98
	rst 38h			;1f99
	rst 38h			;1f9a
	rst 38h			;1f9b
	rst 38h			;1f9c
	rst 38h			;1f9d
	rst 38h			;1f9e
	rst 38h			;1f9f
	rst 38h			;1fa0
	rst 38h			;1fa1
	rst 38h			;1fa2
	rst 38h			;1fa3
	rst 38h			;1fa4
	rst 38h			;1fa5
	rst 38h			;1fa6
	rst 38h			;1fa7
	rst 38h			;1fa8
	rst 38h			;1fa9
	rst 38h			;1faa
	rst 38h			;1fab
	rst 38h			;1fac
	rst 38h			;1fad
	rst 38h			;1fae
	rst 38h			;1faf
	rst 38h			;1fb0
	rst 38h			;1fb1
	rst 38h			;1fb2
	rst 38h			;1fb3
	rst 38h			;1fb4
	rst 38h			;1fb5
	rst 38h			;1fb6
	rst 38h			;1fb7
	rst 38h			;1fb8
	rst 38h			;1fb9
	rst 38h			;1fba
	rst 38h			;1fbb
	rst 38h			;1fbc
	rst 38h			;1fbd
	rst 38h			;1fbe
	rst 38h			;1fbf
	rst 38h			;1fc0
	rst 38h			;1fc1
	rst 38h			;1fc2
	rst 38h			;1fc3
	rst 38h			;1fc4
	rst 38h			;1fc5
	rst 38h			;1fc6
	rst 38h			;1fc7
	rst 38h			;1fc8
	rst 38h			;1fc9
	rst 38h			;1fca
	rst 38h			;1fcb
	rst 38h			;1fcc
	rst 38h			;1fcd
	rst 38h			;1fce
	rst 38h			;1fcf
	rst 38h			;1fd0
	rst 38h			;1fd1
	rst 38h			;1fd2
	rst 38h			;1fd3
	rst 38h			;1fd4
	rst 38h			;1fd5
	rst 38h			;1fd6
	rst 38h			;1fd7
	rst 38h			;1fd8
	rst 38h			;1fd9
	rst 38h			;1fda
	rst 38h			;1fdb
	rst 38h			;1fdc
	rst 38h			;1fdd
	rst 38h			;1fde
	rst 38h			;1fdf
	rst 38h			;1fe0
	rst 38h			;1fe1
	rst 38h			;1fe2
	rst 38h			;1fe3
	rst 38h			;1fe4
	rst 38h			;1fe5
	rst 38h			;1fe6
	rst 38h			;1fe7
	rst 38h			;1fe8
	rst 38h			;1fe9
	rst 38h			;1fea
	rst 38h			;1feb
	rst 38h			;1fec
	rst 38h			;1fed
	rst 38h			;1fee
	rst 38h			;1fef
	rst 38h			;1ff0
	rst 38h			;1ff1
	rst 38h			;1ff2
	rst 38h			;1ff3
	rst 38h			;1ff4
	rst 38h			;1ff5
	rst 38h			;1ff6
	rst 38h			;1ff7
	rst 38h			;1ff8
	rst 38h			;1ff9
	rst 38h			;1ffa
	rst 38h			;1ffb
	rst 38h			;1ffc
	rst 38h			;1ffd
	rst 38h			;1ffe
	rst 38h			;1fff
	rst 38h			;2000
	rst 38h			;2001
	rst 38h			;2002
	rst 38h			;2003
	rst 38h			;2004
	rst 38h			;2005
	rst 38h			;2006
	rst 38h			;2007
	rst 38h			;2008
	rst 38h			;2009
	rst 38h			;200a
	rst 38h			;200b
	rst 38h			;200c
	rst 38h			;200d
	rst 38h			;200e
	rst 38h			;200f
	rst 38h			;2010
	rst 38h			;2011
	rst 38h			;2012
	rst 38h			;2013
	rst 38h			;2014
	rst 38h			;2015
	rst 38h			;2016
	rst 38h			;2017
	rst 38h			;2018
	rst 38h			;2019
	rst 38h			;201a
	rst 38h			;201b
	rst 38h			;201c
	rst 38h			;201d
	rst 38h			;201e
	rst 38h			;201f
	rst 38h			;2020
	rst 38h			;2021
	rst 38h			;2022
	rst 38h			;2023
	rst 38h			;2024
	rst 38h			;2025
	rst 38h			;2026
	rst 38h			;2027
	rst 38h			;2028
	rst 38h			;2029
	rst 38h			;202a
	rst 38h			;202b
	rst 38h			;202c
	rst 38h			;202d
	rst 38h			;202e
	rst 38h			;202f
	rst 38h			;2030
	rst 38h			;2031
	rst 38h			;2032
	rst 38h			;2033
	rst 38h			;2034
	rst 38h			;2035
	rst 38h			;2036
	rst 38h			;2037
	rst 38h			;2038
	rst 38h			;2039
	rst 38h			;203a
	rst 38h			;203b
	rst 38h			;203c
	rst 38h			;203d
	rst 38h			;203e
	rst 38h			;203f
	rst 38h			;2040
	rst 38h			;2041
	rst 38h			;2042
	rst 38h			;2043
	rst 38h			;2044
	rst 38h			;2045
	rst 38h			;2046
	rst 38h			;2047
	rst 38h			;2048
	rst 38h			;2049
	rst 38h			;204a
	rst 38h			;204b
	rst 38h			;204c
	rst 38h			;204d
	rst 38h			;204e
	rst 38h			;204f
	rst 38h			;2050
	rst 38h			;2051
	rst 38h			;2052
	rst 38h			;2053
	rst 38h			;2054
	rst 38h			;2055
	rst 38h			;2056
	rst 38h			;2057
	rst 38h			;2058
	rst 38h			;2059
	rst 38h			;205a
	rst 38h			;205b
	rst 38h			;205c
	rst 38h			;205d
	rst 38h			;205e
	rst 38h			;205f
	rst 38h			;2060
	rst 38h			;2061
	rst 38h			;2062
	rst 38h			;2063
	rst 38h			;2064
	rst 38h			;2065
	rst 38h			;2066
	rst 38h			;2067
	rst 38h			;2068
	rst 38h			;2069
	rst 38h			;206a
	rst 38h			;206b
	rst 38h			;206c
	rst 38h			;206d
	rst 38h			;206e
	rst 38h			;206f
	rst 38h			;2070
	rst 38h			;2071
	rst 38h			;2072
	rst 38h			;2073
	rst 38h			;2074
	rst 38h			;2075
	rst 38h			;2076
	rst 38h			;2077
	rst 38h			;2078
	rst 38h			;2079
	rst 38h			;207a
	rst 38h			;207b
	rst 38h			;207c
	rst 38h			;207d
	rst 38h			;207e
	rst 38h			;207f
	rst 38h			;2080
	rst 38h			;2081
	rst 38h			;2082
	rst 38h			;2083
	rst 38h			;2084
	rst 38h			;2085
	rst 38h			;2086
	rst 38h			;2087
	rst 38h			;2088
	rst 38h			;2089
	rst 38h			;208a
	rst 38h			;208b
	rst 38h			;208c
	rst 38h			;208d
	rst 38h			;208e
	rst 38h			;208f
	rst 38h			;2090
	rst 38h			;2091
	rst 38h			;2092
	rst 38h			;2093
	rst 38h			;2094
	rst 38h			;2095
	rst 38h			;2096
	rst 38h			;2097
	rst 38h			;2098
	rst 38h			;2099
	rst 38h			;209a
	rst 38h			;209b
	rst 38h			;209c
	rst 38h			;209d
	rst 38h			;209e
	rst 38h			;209f
	rst 38h			;20a0
	rst 38h			;20a1
	rst 38h			;20a2
	rst 38h			;20a3
	rst 38h			;20a4
	rst 38h			;20a5
	rst 38h			;20a6
	rst 38h			;20a7
	rst 38h			;20a8
	rst 38h			;20a9
	rst 38h			;20aa
	rst 38h			;20ab
	rst 38h			;20ac
	rst 38h			;20ad
	rst 38h			;20ae
	rst 38h			;20af
	rst 38h			;20b0
	rst 38h			;20b1
	rst 38h			;20b2
	rst 38h			;20b3
	rst 38h			;20b4
	rst 38h			;20b5
	rst 38h			;20b6
	rst 38h			;20b7
	rst 38h			;20b8
	rst 38h			;20b9
	rst 38h			;20ba
	rst 38h			;20bb
	rst 38h			;20bc
	rst 38h			;20bd
	rst 38h			;20be
	rst 38h			;20bf
	rst 38h			;20c0
	rst 38h			;20c1
	rst 38h			;20c2
	rst 38h			;20c3
	rst 38h			;20c4
	rst 38h			;20c5
	rst 38h			;20c6
	rst 38h			;20c7
	rst 38h			;20c8
	rst 38h			;20c9
	rst 38h			;20ca
	rst 38h			;20cb
	rst 38h			;20cc
	rst 38h			;20cd
	rst 38h			;20ce
	rst 38h			;20cf
	rst 38h			;20d0
	rst 38h			;20d1
	rst 38h			;20d2
	rst 38h			;20d3
	rst 38h			;20d4
	rst 38h			;20d5
	rst 38h			;20d6
	rst 38h			;20d7
	rst 38h			;20d8
	rst 38h			;20d9
	rst 38h			;20da
	rst 38h			;20db
	rst 38h			;20dc
	rst 38h			;20dd
	rst 38h			;20de
	rst 38h			;20df
	rst 38h			;20e0
	rst 38h			;20e1
	rst 38h			;20e2
	rst 38h			;20e3
	rst 38h			;20e4
	rst 38h			;20e5
	rst 38h			;20e6
	rst 38h			;20e7
	rst 38h			;20e8
	rst 38h			;20e9
	rst 38h			;20ea
	rst 38h			;20eb
	rst 38h			;20ec
	rst 38h			;20ed
	rst 38h			;20ee
	rst 38h			;20ef
	rst 38h			;20f0
	rst 38h			;20f1
	rst 38h			;20f2
	rst 38h			;20f3
	rst 38h			;20f4
	rst 38h			;20f5
	rst 38h			;20f6
	rst 38h			;20f7
	rst 38h			;20f8
	rst 38h			;20f9
	rst 38h			;20fa
	rst 38h			;20fb
	rst 38h			;20fc
	rst 38h			;20fd
	rst 38h			;20fe
	rst 38h			;20ff
