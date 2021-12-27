package studchain

import (
	"bytes"
	"encoding/binary"
	"fmt"
	"log"
	"sync"
	"time"

	"golang.org/x/crypto/sha3"

	"gitlab.com/pnathan/studchain/src/lib"
)
type Hashtype []byte


type Block struct {
	Index        uint64
	PreviousHash Hashtype
	Timestamp    time.Time
	Hash         Hashtype
	Data         []byte
}

func (b Block) Equal(other Block) bool {

	return b.Index == other.Index &&
		bytes.Equal(b.PreviousHash, other.PreviousHash) &&
		b.Timestamp == other.Timestamp &&
		bytes.Equal(b.Hash, other.Hash) &&
		bytes.Equal(b.Data, b.Data)
}

func (b Block) String() string {
	return fmt.Sprintf("> %v | %v | %x | %x | %v", b.Index, b.Timestamp.Unix(), b.PreviousHash, b.Hash, string(b.Data))
}

func UintToBytes(u uint64) []byte {
	int_buffer := make([]byte, binary.MaxVarintLen64)
	n := binary.PutUvarint(int_buffer, u)
	return int_buffer[:n]
}

func IntToBytes(u int64) []byte {
	int_buffer := make([]byte, binary.MaxVarintLen64)
	n := binary.PutVarint(int_buffer, u)
	return int_buffer[:n]
}

func blocklessHash(index uint64, timestamp time.Time, prior Hashtype, data []byte) (Hashtype, error) {

	buf := lib.Concat(UintToBytes(index), []byte(prior), IntToBytes(timestamp.Unix()), data)

	sha3.NewShake256()
	h := make([]byte, 64)
	// Compute a 64-byte Hash of buf and put it in h.
	sha3.ShakeSum256(h, buf)
	return Hashtype(h), nil
}

func (b Block) CalculateHash() (Hashtype, error) {
	return blocklessHash(b.Index, b.Timestamp, b.PreviousHash, b.Data)
}

func Genesis() *Block {
	idx := uint64(0)
	starter := "בְּרֵאשִׁ֖ית בָּרָ֣א"
	priorHash := []byte{0xDE, 0xEA, 0xD, 0xBE, 0xFF}
	genesisTime := time.Date(1, 1, 1, 1, 1, 1, 1, time.UTC)
	hash, err := blocklessHash(idx, genesisTime, priorHash, []byte(starter))
	if err != nil {
		m := "omg can't Hash"
		panic(m)
	}
	return &Block{
		Index:        idx,
		PreviousHash: priorHash,
		Timestamp:    genesisTime,
		Hash:         hash,
		Data:         []byte(starter),
	}
}

func ValidateBlock(b *Block) bool {
	candidateHash, err := blocklessHash(b.Index, b.Timestamp, b.PreviousHash, b.Data)
	if err != nil {
		return false
	}
	if !bytes.Equal(candidateHash, b.Hash) {
		return false
	}
	return true
}

type Chain struct {
	List  []Block      `json:"blocks"`
	Mutex sync.RWMutex `json:"-"`
}

func (c Chain) Length() int {
	return len(c.List)
}

func (c *Chain) ValidateChain() bool {
	c.Mutex.RLock()
	defer c.Mutex.RUnlock()
	if ! c.List[0].Equal(*Genesis()) {
		log.Printf("genesis mismatch: candidate %v | genesis: %v", c.List[0], Genesis())
		return false
	}
	for idx, element := range c.List {
		if idx == 0 {
			continue
		}
		log.Printf("Checking index %d", idx)
		// Is the block true to itself?
		if !ValidateBlock(&element) {
			log.Printf("block mismatch: %d", idx)
			return false
		}
		prior := c.List[idx-1]
		if element.Index-1 != prior.Index {
			return false
		}
		// Is the prior pointer correct?
		if !bytes.Equal(element.PreviousHash, prior.Hash) {
			return false
		}
	}
	return true
}

func (c *Chain) GetList() []*Block {
	c.Mutex.RLock()
	defer c.Mutex.RUnlock()
	blocks := []*Block{}
	for _, e := range c.List {
		cloned := &Block{
			Index:        e.Index,
			PreviousHash: e.PreviousHash,
			Timestamp:    e.Timestamp,
			Hash:         e.Hash,
			Data:         e.Data,
		}

		blocks = append(blocks, cloned)
	}
	return blocks
}

func (c *Chain) AppendBlock(data []byte) error {
	c.Mutex.Lock()
	defer c.Mutex.Unlock()
	sz := uint64(len(c.List))
	prior := c.List[sz-1]
	time_of_generation := time.Now()
	newhash, err := blocklessHash(sz, time_of_generation, prior.Hash, data)
	if err != nil {
		return err
	}
	c.List = append(c.List,
		Block{
			Index:        sz,
			PreviousHash: prior.Hash,
			Timestamp:    time_of_generation,
			Hash:         newhash,
			Data:         data,
		})

	return nil
}
