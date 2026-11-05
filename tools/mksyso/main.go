// mksyso: собирает rsrc_windows_amd64.syso с иконкой приложения из набора PNG.
// Использование: mksyso out.syso 16.png 32.png 48.png 256.png
package main

import (
	"bytes"
	"encoding/binary"
	"image/png"
	"os"
)

type res struct {
	typ, id uint32
	data    []byte
}

func le(b *bytes.Buffer, v ...interface{}) {
	for _, x := range v {
		binary.Write(b, binary.LittleEndian, x)
	}
}

func main() {
	out := os.Args[1]
	var resources []res
	var grp bytes.Buffer
	le(&grp, uint16(0), uint16(1), uint16(len(os.Args)-2))
	for i, p := range os.Args[2:] {
		data, err := os.ReadFile(p)
		if err != nil {
			panic(err)
		}
		cfg, err := png.DecodeConfig(bytes.NewReader(data))
		if err != nil {
			panic(err)
		}
		w, h := cfg.Width, cfg.Height
		if w >= 256 {
			w = 0
		}
		if h >= 256 {
			h = 0
		}
		id := uint32(i + 1)
		resources = append(resources, res{3, id, data})
		le(&grp, uint8(w), uint8(h), uint8(0), uint8(0), uint16(1), uint16(32), uint32(len(data)), uint16(id))
	}
	resources = append(resources, res{14, 1, grp.Bytes()})

	// типы по возрастанию: 3 (RT_ICON), 14 (RT_GROUP_ICON)
	types := []uint32{3, 14}
	byType := map[uint32][]res{}
	for _, r := range resources {
		byType[r.typ] = append(byType[r.typ], r)
	}

	// раскладка
	rootSize := 16 + 8*len(types)
	off := rootSize
	typeOff := map[uint32]int{}
	for _, t := range types {
		typeOff[t] = off
		off += 16 + 8*len(byType[t])
	}
	langOff := make([]int, len(resources))
	idx := 0
	order := []res{}
	for _, t := range types {
		for _, r := range byType[t] {
			langOff[idx] = off
			off += 24
			order = append(order, r)
			idx++
		}
	}
	dataEntryOff := make([]int, len(order))
	for i := range order {
		dataEntryOff[i] = off
		off += 16
	}
	blobOff := make([]int, len(order))
	for i, r := range order {
		off = (off + 7) &^ 7
		blobOff[i] = off
		off += len(r.data)
	}
	total := (off + 7) &^ 7

	raw := make([]byte, total)
	put16 := func(o int, v uint16) { binary.LittleEndian.PutUint16(raw[o:], v) }
	put32 := func(o int, v uint32) { binary.LittleEndian.PutUint32(raw[o:], v) }
	dir := func(o, n int) { put16(o+14, uint16(n)) } // только ID-записи

	dir(0, len(types))
	for i, t := range types {
		put32(16+8*i, t)
		put32(16+8*i+4, uint32(typeOff[t])|0x80000000)
	}
	idx = 0
	for _, t := range types {
		o := typeOff[t]
		dir(o, len(byType[t]))
		for j, r := range byType[t] {
			put32(o+16+8*j, r.id)
			put32(o+16+8*j+4, uint32(langOff[idx])|0x80000000)
			idx++
		}
	}
	var relocs []uint32
	for i, r := range order {
		o := langOff[i]
		dir(o, 1)
		put32(o+16, 0x0409)
		put32(o+20, uint32(dataEntryOff[i]))
		d := dataEntryOff[i]
		put32(d, uint32(blobOff[i])) // RVA — правится релокацией
		put32(d+4, uint32(len(r.data)))
		relocs = append(relocs, uint32(d))
		copy(raw[blobOff[i]:], r.data)
	}

	var f bytes.Buffer
	const hdr, sec = 20, 40
	rawPtr := hdr + sec
	relPtr := rawPtr + len(raw)
	symPtr := relPtr + 10*len(relocs)
	// COFF header
	le(&f, uint16(0x8664), uint16(1), uint32(0), uint32(symPtr), uint32(1), uint16(0), uint16(0))
	// section header
	f.WriteString(".rsrc\x00\x00\x00")
	le(&f, uint32(0), uint32(0), uint32(len(raw)), uint32(rawPtr), uint32(relPtr), uint32(0), uint16(len(relocs)), uint16(0), uint32(0x40000040))
	f.Write(raw)
	for _, r := range relocs {
		le(&f, r, uint32(0), uint16(3)) // IMAGE_REL_AMD64_ADDR32NB
	}
	// symbol .rsrc
	f.WriteString(".rsrc\x00\x00\x00")
	le(&f, uint32(0), int16(1), uint16(0), uint8(3), uint8(0))
	le(&f, uint32(4)) // пустая таблица строк
	if err := os.WriteFile(out, f.Bytes(), 0644); err != nil {
		panic(err)
	}
}
