package tiaf

import (
	"sync"

	"go.uber.org/zap"

	"gitlab.com/pnathan/tiaf/src/lib/log"
	"gitlab.com/pnathan/tiaf/src/lib/tiafapi"
	"gitlab.com/pnathan/tiaf/src/lib/utility/trie"
)

type InternalChain struct {
	tiafapi.Chain
	Mutex       sync.RWMutex
	SeenRecords *trie.Trie
}

func NewChain() *InternalChain {
	genesis := tiafapi.Genesis()
	seen := []string{}
	for _, items := range genesis.Data.Items {
		seen = append(seen, string(items.GetHash()))
	}
	return &InternalChain{
		Chain:       tiafapi.Chain{BlockList: []tiafapi.Block{*genesis}},
		SeenRecords: trie.New(seen),
	}
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

func (chain *InternalChain) SwapIn(chain2 tiafapi.Chain) {

	chain.Mutex.Lock()
	defer chain.Mutex.Unlock()
	chain.BlockList = chain2.BlockList

	// recache all record entries.
	if err := chain.ensureHistoryUnlocked(); err != nil {
		log.Info("Error in generating history", zap.Error(err))
	}

}
func (chain *InternalChain) ensureHistoryUnlocked() error {
	for _, block := range chain.Chain.BlockList {
		for _, item := range block.Data.Items {
			chain.SeenRecords.Put(string(item.GetHash()))
		}
	}
	return nil
}
func (chain *InternalChain) EnsureHistory() error {
	chain.Mutex.Lock()
	defer chain.Mutex.Unlock()
	return chain.ensureHistoryUnlocked()
}

func (chain *InternalChain) HasSeen(data tiafapi.Record) (bool, error) {
	chain.Mutex.RLock()
	defer chain.Mutex.RUnlock()
	log.Info("Checking to see if I've already seen a record in the chain...")
	return chain.SeenRecords.Exist(string(data.GetHash())), nil
}

func (chain *InternalChain) AppendBlock(data tiafapi.RecordCollection) error {
	chain.Mutex.Lock()
	defer chain.Mutex.Unlock()
	for _, record := range data.Items {
		chain.SeenRecords.Put(string(record.GetHash()))
	}
	return chain.Chain.AppendBlock(data)
}
