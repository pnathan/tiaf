package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"io/ioutil"
	"os"

	"github.com/akamensky/argparse"
	"go.uber.org/zap"

	"gitlab.com/pnathan/tiaf/src/lib/log"
	"gitlab.com/pnathan/tiaf/src/lib/tiafapi"
)

func MustMarshal(v any) []byte {
	b := new(bytes.Buffer)
	encoder := json.NewEncoder(b)
	encoder.SetIndent("", "  ")
	err := encoder.Encode(v)
	if err != nil {
		panic(err)
	}

	return b.Bytes()
}

func Moan(complaint error) {
	log.Fatal("", zap.Error(complaint))
	os.Exit(1)
}

func main() {
	parser := argparse.NewParser("tiaf client", "tiaf client code")

	endpoint := parser.String("e", "endpoint", &argparse.Options{Required: true, Help: "endpoint to address", Default: "localhost:1337"})

	// Note: subcommands useful here, but do later.
	getChainCmd := parser.NewCommand("chain-get", "get chain")

	putChainCmd := parser.NewCommand("chain-put", "put chain")

	file := putChainCmd.String("f", "file", &argparse.Options{Required: true, Help: "file using"})
	compareChainCmd := parser.NewCommand("chain-compare", "compare chain")

	putDataCmd := parser.NewCommand("block-append", "puts data in chain")
	fileData := putDataCmd.String("f", "file", &argparse.Options{Required: false, Help: "file with the data; if not present, reads from stdin"})

	peerPut := parser.NewCommand("peer-put", "puts the peer list")
	peerFile := peerPut.String("f", "file", &argparse.Options{Required: true, Help: "list of the peers"})
	peerGet := parser.NewCommand("peer-get", "gets the peer list")
	peerSweep := parser.NewCommand("peer-sweep", "request a sweep")
	peerSweepEnable := parser.NewCommand("peer-sweep-enable", "enable automatic sweeps")
	peerSweepDisable := parser.NewCommand("peer-sweep-enable", "disable automatic sweeps")

	// Parse input
	err := parser.Parse(os.Args)
	if err != nil {
		// In case of error print error and print usage
		// This can also be done by passing -h or --help flags
		fmt.Print(parser.Usage(err))
		return
	}

	if putChainCmd.Happened() {
		data, err := ioutil.ReadFile(*file)
		if err != nil {
			Moan(err)
		}
		c := &tiafapi.Chain{}
		if err := json.Unmarshal(data, c); err != nil {
			Moan(err)
		}
		if err := tiafapi.PutChain(*c, *endpoint); err != nil {
			Moan(err)
		}
	} else if getChainCmd.Happened() {
		c, err := tiafapi.GetChain(*endpoint)
		if err != nil {
			Moan(err)
		}
		fmt.Println(string(MustMarshal(c)))
	} else if compareChainCmd.Happened() {
		fmt.Println("not implemented")
	} else if putDataCmd.Happened() {
		var input string
		if *fileData != "" {
			filedata, err := ioutil.ReadFile(*fileData)
			if err != nil {
				log.Fatal("unable to read file", zap.String("filename", *fileData), zap.Error(err))
			}
			input = string(filedata)
		} else {
			sin, err := io.ReadAll(os.Stdin)
			if err != nil {
				Moan(err)
			}
			input = string(sin)
		}
		data := &tiafapi.BlockData{Data: input}
		if err := tiafapi.AppendBlock(data, *endpoint); err != nil {
			Moan(err)
		}
	} else if peerPut.Happened() {
		filedata, err := ioutil.ReadFile(*peerFile)
		if err != nil {
			Moan(err)
		}
		peers := &tiafapi.Peerage{}
		if err := json.Unmarshal(filedata, peers); err != nil {
			Moan(err)
		}
		if err := tiafapi.PutPeers(peers, *endpoint); err != nil {
			Moan(err)
		}
	} else if peerGet.Happened() {
		peers, err := tiafapi.GetPeers(*endpoint)
		if err != nil {
			Moan(err)
		}
		fmt.Println(string(MustMarshal(peers)))
	} else if peerSweep.Happened() {
		if err := tiafapi.PostSweep(*endpoint); err != nil {
			Moan(err)
		}
	} else if peerSweepEnable.Happened() {
		if err := tiafapi.PutAutoSweeps(*endpoint); err != nil {
			Moan(err)
		}
	} else if peerSweepDisable.Happened() {
		if err := tiafapi.DeleteAutoSweeps(*endpoint); err != nil {
			Moan(err)
		}
	} else {
		Moan(fmt.Errorf("can't happen"))
	}
}
