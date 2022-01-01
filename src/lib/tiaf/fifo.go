package tiaf

import (
	"fmt"
	"sync"

	"gitlab.com/pnathan/tiaf/src/lib/tiafapi"
)

const maxSize = 100_000

// Good candidate for generics...
type Fifo struct {
	r      [maxSize]*tiafapi.Record
	start  int
	end    int
	length int
	sync.RWMutex
}

func NewFifo() *Fifo {
	return &Fifo{
		r:       [maxSize]*tiafapi.Record{},
		start:   0,
		end:     0,
		length:  0,
		RWMutex: sync.RWMutex{},
	}
}

func (f *Fifo) Put(r *tiafapi.Record) error {
	if f.length >= maxSize {
		return fmt.Errorf("full")
	}

	f.r[f.end] = r
	f.end++
	f.length++
	return nil
}

func (f *Fifo) Length() int {
	return f.length
}

func (f *Fifo) Pop() (*tiafapi.Record, error) {
	if f.length <= 0 {
		return nil, nil
	}
	r := f.r[f.start]
	f.start++
	f.length--

	return r, nil
}
