package tiaf

import (
	"sync"

	"gitlab.com/pnathan/tiaf/src/lib/tiafapi"
)

type InternalChain struct {
	tiafapi.Chain
	Mutex sync.RWMutex
}

func (chain *InternalChain) Length() int {
	chain.Mutex.RLock()
	defer chain.Mutex.RUnlock()
	return chain.Chain.Length()
}

func (chain *InternalChain) ValidateChain() bool {
	chain.Mutex.RLock()
	defer chain.Mutex.RUnlock()
	return chain.Chain.ValidateChain()
}

func (chain *InternalChain) GetList() []*tiafapi.Block {
	chain.Mutex.RLock()
	defer chain.Mutex.RUnlock()
	blocks := []*tiafapi.Block{}
	// deep copy.
	for _, e := range chain.BlockList {
		cloned := &tiafapi.Block{
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

func (chain *InternalChain) AppendBlock(data tiafapi.Datatype) error {
	chain.Mutex.Lock()
	defer chain.Mutex.Unlock()
	return chain.Chain.AppendBlock(data)
}
