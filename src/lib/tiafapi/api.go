package tiafapi

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"time"

	"go.uber.org/zap"
	"golang.org/x/crypto/sha3"

	"gitlab.com/pnathan/tiaf/src/lib/log"

	"gitlab.com/pnathan/tiaf/src/lib/utility"
)

// BlockData is a serialization structure
type BlockData struct {
	Data string `json:"data"`
}

// Chain is the structure used for serialization
type Chain struct {
	BlockList []Block `json:"block_list"`
}

func (chain *Chain) Length() int {
	return len(chain.BlockList)
}

func (chain *Chain) ValidateChain() bool {

	initialBlock := chain.BlockList[0]
	if !initialBlock.Equal(*Genesis()) {
		log.Printf("genesis mismatch: candidate %v | genesis: %v", chain.BlockList[0], Genesis())
		return false
	}
	for idx, element := range chain.BlockList {
		if idx == 0 {
			continue
		}
		log.Printf("Checking index %d", idx)
		// Is the block true to itself?
		if !element.ValidateBlock() {
			log.Printf("block mismatch: %d", idx)
			return false
		}
		prior := chain.BlockList[idx-1]
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

func (chain *Chain) AppendBlock(data Datatype) error {
	sz := uint64(len(chain.BlockList))
	prior := chain.BlockList[sz-1]
	timeOfGeneration := time.Now()
	newhash, err := CalculateHash(sz, timeOfGeneration, prior.Hash, data)
	if err != nil {
		return err
	}
	chain.BlockList = append(chain.BlockList,
		Block{
			Index:        sz,
			PreviousHash: prior.Hash,
			Timestamp:    timeOfGeneration,
			Hash:         newhash,
			Data:         data,
		})
	return nil
}

type Hashtype []byte

type Datatype string

// Block is the serialization of a block.
type Block struct {
	Index        uint64    `json:"index"`
	PreviousHash Hashtype  `json:"previous_hash"`
	Timestamp    time.Time `json:"timestamp"`
	Hash         Hashtype  `json:"hash"`
	Data         Datatype  `json:"data"`
}

func Genesis() *Block {
	idx := uint64(0)
	starter := "בְּרֵאשִׁ֖ית בָּרָ֣א"
	priorHash := []byte{0xDE, 0xEA, 0xD, 0xBE, 0xFF}
	genesisTime := time.Date(1, 1, 1, 1, 1, 1, 1, time.UTC)
	hash, err := CalculateHash(idx, genesisTime, priorHash, Datatype(starter))
	if err != nil {
		log.Fatal("System failure: unable to validate hash of Genesis block", zap.Error(err))
	}
	return &Block{
		Index:        idx,
		PreviousHash: priorHash,
		Timestamp:    genesisTime,
		Hash:         hash,
		Data:         Datatype(starter),
	}
}

func (b *Block) ValidateBlock() bool {
	candidateHash, err := CalculateHash(b.Index, b.Timestamp, b.PreviousHash, b.Data)
	if err != nil {
		return false
	}
	if !bytes.Equal(candidateHash, b.Hash) {
		return false
	}
	return true
}

func (b Block) Equal(other Block) bool {
	return b.Index == other.Index &&
		bytes.Equal(b.PreviousHash, other.PreviousHash) &&
		b.Timestamp == other.Timestamp &&
		bytes.Equal(b.Hash, other.Hash) &&
		b.Data == b.Data
}

func (b Block) String() string {
	return fmt.Sprintf("> %v | %v | %x | %x | %v", b.Index, b.Timestamp.Unix(), b.PreviousHash, b.Hash, string(b.Data))
}

func (b Block) CalculateHash() (Hashtype, error) {
	return CalculateHash(b.Index, b.Timestamp, b.PreviousHash, b.Data)
}

func CalculateHash(index uint64, timestamp time.Time, prior Hashtype, data Datatype) (Hashtype, error) {
	buf := utility.Concat(utility.UintToBytes(index), []byte(prior), utility.IntToBytes(timestamp.Unix()), []byte(data))

	sha3.NewShake256()
	h := make([]byte, 64)
	// Compute a 64-byte Hash of buf and put it in h.
	sha3.ShakeSum256(h, buf)
	return h, nil
}

type Peerage struct {
	Peers []string `json:"peers"`
}

const (
	http_put    = "PUT"
	http_delete = "DELETE"
)

func httpPut(addr string, text []byte) (*http.Response, error) {
	return httpMethod(http_put, addr, text)
}

func httpDelete(addr string, text []byte) (*http.Response, error) {
	return httpMethod(http_delete, addr, text)
}

func httpMethod(method, addr string, text []byte) (*http.Response, error) {
	log.Info("Reading peer", zap.String("endpoint", addr))
	buf := bytes.NewBuffer(text)
	client := &http.Client{}
	req, err := http.NewRequest(method, addr, buf)
	if err != nil {
		log.Warn("http error", zap.Error(err))
		return nil, err
	}
	resp, err := client.Do(req)
	if err != nil {
		log.Warn("http error", zap.Error(err))
		return nil, err
	}

	return resp, nil
}

func PutPeers(data *Peerage, addr string) error {
	text, err := json.Marshal(data)
	if err != nil {

		return err
	}
	formulatedAddress := fmt.Sprintf("%v/api/peers", addr)
	resp, err := httpPut(formulatedAddress, text)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	switch resp.StatusCode {
	case http.StatusBadRequest:
		return fmt.Errorf("bad request made, erroring")
	case http.StatusOK:
	}
	return nil
}

func GetPeers(addr string) (*Peerage, error) {
	formulatedAddress := fmt.Sprintf("%v/api/peers", addr)
	resp, err := http.Get(formulatedAddress)
	if err != nil {
		log.Warn("http error", zap.Error(err))
		return nil, err
	}
	defer resp.Body.Close()
	switch resp.StatusCode {
	case http.StatusBadRequest:
		return nil, fmt.Errorf("bad request made, erroring")
	case http.StatusOK:
	}

	decoder := json.NewDecoder(resp.Body)

	s := &Peerage{}
	if err := decoder.Decode(s); err != nil {
		log.Warn("decoding error", zap.Error(err), zap.String("address", formulatedAddress))
		return nil, err
	}
	return s, nil
}

func PostSweep(addr string) error {
	formulatedAddress := fmt.Sprintf("%v/api/peers/sweep", addr)
	resp, err := http.Post(formulatedAddress, "application/json", nil)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("bad error code: %d", resp.StatusCode)
	}

	return nil
}

func PutAutoSweeps(addr string) error {
	formulatedAddress := fmt.Sprintf("%v/api/peers/sweep/auto", addr)
	resp, err := httpPut(formulatedAddress, []byte{})
	if err != nil {
		return err
	}
	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("bad error code: %d", resp.StatusCode)
	}
	return nil
}
func DeleteAutoSweeps(addr string) error {
	formulatedAddress := fmt.Sprintf("%v/api/peers/sweep/auto", addr)
	resp, err := httpDelete(formulatedAddress, []byte{})
	if err != nil {
		return err
	}
	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("bad error code: %d", resp.StatusCode)
	}
	return nil
}

// AppendBlock writes the data in b to the chain, in a block.
// If multiple data pieces are to be written, that is to be gathered into the data in b.
func AppendBlock(data *BlockData, addr string) error {
	text, err := json.Marshal(data)
	if err != nil {

		return err
	}
	formulatedAddress := fmt.Sprintf("%v/api/block", addr)

	resp, err := httpPut(formulatedAddress, text)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	switch resp.StatusCode {
	case http.StatusInternalServerError:
		fallthrough
	case http.StatusNotAcceptable:
		fallthrough
	case http.StatusBadRequest:
		fmt.Printf("Unable to append data")
		return fmt.Errorf("error in putting data")
	case http.StatusOK:
		fmt.Printf("completed")
	}

	return nil
}

func PutChain(c Chain, addr string) error {
	text, err := json.Marshal(c)
	if err != nil {
		return err
	}
	formulatedAddress := fmt.Sprintf("%v/api/chain", addr)

	resp, err := httpPut(formulatedAddress, text)
	if err != nil {
		log.Printf("error reading peer %v", err)
		return err
	}
	defer resp.Body.Close()

	switch resp.StatusCode {
	case http.StatusBadRequest:
		return fmt.Errorf("bad request")
	case http.StatusCreated:
	case http.StatusOK:
	}

	return nil
}

func GetChain(addr string) (*Chain, error) {
	formulatedAddress := fmt.Sprintf("%v/api/chain", addr)
	log.Info("Reading peer", zap.String("endpoint", formulatedAddress))

	resp, err := http.Get(formulatedAddress)
	if err != nil {
		log.Warn("http error", zap.Error(err))
		return nil, err
	}
	defer resp.Body.Close()

	decoder := json.NewDecoder(resp.Body)

	candidateChain := &Chain{}
	if err := decoder.Decode(candidateChain); err != nil {
		log.Warn("decoding error", zap.Error(err), zap.String("address", formulatedAddress))
		return nil, err
	}
	return candidateChain, err
}
