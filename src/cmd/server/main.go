package main

import (
	"encoding/json"
	"fmt"
	"io/ioutil"
	"math/rand"
	"net/http"
	"net/url"
	"os"
	"sync"
	"time"

	"github.com/akamensky/argparse"
	"github.com/gorilla/mux"
	"github.com/justinas/alice"
	"go.uber.org/zap"

	"gitlab.com/pnathan/tiaf/src/lib/log"
	"gitlab.com/pnathan/tiaf/src/lib/tiaf"
	"gitlab.com/pnathan/tiaf/src/lib/tiafapi"
	"gitlab.com/pnathan/tiaf/src/lib/utility/trie"
)

var GLOBAL_CHAIN *tiaf.InternalChain

func returnChain(w http.ResponseWriter, r *http.Request) {

	bytes, err := json.Marshal(GLOBAL_CHAIN)
	if err != nil {
		log.Printf("error: %v", err)
		w.WriteHeader(http.StatusInternalServerError)
		return
	}
	fmt.Fprint(w, string(bytes))
}

func appendBlock(w http.ResponseWriter, r *http.Request) {
	decoder := json.NewDecoder(r.Body)

	data := tiafapi.BlockData{}
	if err := decoder.Decode(&data); err != nil {
		w.WriteHeader(http.StatusBadRequest)
		_, _ = w.Write([]byte("query parse fail"))

		return
	}
	log.Printf("data: %v", data)
	if string(data.Data) == "" {
		w.WriteHeader(http.StatusNotAcceptable)
		_, _ = w.Write([]byte("empty data!"))

		return
	}

	record := []tiafapi.Record{{Timestamp: time.Now().Unix(), Entry: data.Data}}
	// TODO: validate this data isn't a double-put.
	// if ! HasSeen(record)...
	if err := GLOBAL_CHAIN.AppendBlock(tiafapi.RecordCollection{Items: record}); err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		log.Printf("%v", err)
		_, _ = w.Write([]byte("error"))
		return
	}
	_, _ = w.Write([]byte("ok"))
}

func evaluateNewChainForAcceptance(w http.ResponseWriter, r *http.Request) {
	decoder := json.NewDecoder(r.Body)

	candidateChain := tiafapi.Chain{}
	if err := decoder.Decode(&candidateChain); err != nil {
		log.Printf("%#v", err)

		w.WriteHeader(http.StatusBadRequest)
		_, _ = w.Write([]byte("couldn't decode"))
		return
	}

	if !candidateChain.ValidateChain() {
		w.WriteHeader(http.StatusBadRequest)
		_, _ = w.Write([]byte("invalid chain submitted"))
		return
	}

	// this is actually very silly!
	// https://lhartikk.github.io/jekyll/update/2017/07/14/chapter1.html
	// Really? Longest chain?
	if GLOBAL_CHAIN.Length() < candidateChain.Length() {
		GLOBAL_CHAIN.SwapIn(candidateChain)

		w.WriteHeader(http.StatusCreated)
		_, _ = w.Write([]byte("replaced with new chain"))

	} else {
		_, _ = w.Write([]byte("no change"))
	}
}

func measureOtherChain(w http.ResponseWriter, r *http.Request) {
	decoder := json.NewDecoder(r.Body)

	candidateChain := tiafapi.Chain{}
	if err := decoder.Decode(&candidateChain); err != nil {
		log.Printf("%#v", err)
		w.WriteHeader(http.StatusBadRequest)
		_, _ = w.Write([]byte("couldn't decode"))
		return
	}

	if !candidateChain.ValidateChain() {
		w.WriteHeader(http.StatusBadRequest)
		_, _ = w.Write([]byte("invalid chain submitted"))
		return
	}

	if GLOBAL_CHAIN.Length() < candidateChain.Length() {
		w.WriteHeader(http.StatusAccepted)
		_, _ = w.Write([]byte("submitted chain is newer"))
	} else {
		_, _ = w.Write([]byte("contained chain is at least equal, if not newer"))
	}
}

var GLOBAL_PEERS *tiaf.InternalPeers

func putPeers(w http.ResponseWriter, r *http.Request) {
	decoder := json.NewDecoder(r.Body)

	peers := tiafapi.Peerage{}
	if err := decoder.Decode(&peers); err != nil {

		w.WriteHeader(http.StatusBadRequest)
		_, _ = w.Write([]byte("couldn't decode"))
		return
	}

	// TODO: refactor into function
	GLOBAL_PEERS.Lock()
	defer GLOBAL_PEERS.Unlock()
	GLOBAL_PEERS.Peers = peers.Peers

	_, _ = w.Write([]byte("ok"))
}

func getPeers(w http.ResponseWriter, r *http.Request) {
	peers := GLOBAL_PEERS.GetPeers()
	text, err := json.Marshal(peers)
	if err != nil {
		w.WriteHeader(http.StatusBadRequest)
		_, _ = w.Write([]byte("err"))
		return
	}
	_, _ = w.Write(text)
}

// sweepPeers is a pull based chain getter.
func sweepPeers(w http.ResponseWriter, r *http.Request) {
	internalSweeper()
}

func internalSweeper() []string {
	endpoints := GLOBAL_PEERS.GetPeers()
	retval := []string{}
	for _, addr := range endpoints {
		sweepOnePeer(addr)
	}
	return retval
}

func sweepOnePeer(addr string) {
	candidateChain, err := tiafapi.GetChain(addr)
	if err != nil {
		log.Printf("error getting chain: %v", err)
		return
	}

	if !candidateChain.ValidateChain() {
		log.Printf("invalid chain gotten from %v", addr)
		return
	}

	if GLOBAL_CHAIN.Length() < candidateChain.Length() {
		log.Info("Updating from peer", zap.String("host", addr))
		GLOBAL_CHAIN.SwapIn(*candidateChain)
		log.Printf("updated from %v", addr)
	}
}

type SweepState int64

const (
	WillSweep SweepState = iota
	WillNotSweep
)

type doSweep struct {
	sync.Mutex
	state SweepState
}

func (s *doSweep) IsSweeping() bool {
	s.Lock()
	defer s.Unlock()
	return s.state == WillSweep
}
func (s *doSweep) EnableSweeping() {
	s.Lock()
	defer s.Unlock()
	s.state = WillSweep
}
func (s *doSweep) DisableSweeping() {
	s.Lock()
	defer s.Unlock()
	s.state = WillSweep
}

var GLOBAL_CURRENT_SWEEP_STATE *doSweep

func autoSweepPeersEnable(w http.ResponseWriter, r *http.Request) {
	log.Printf("enabling sweeping")
	GLOBAL_CURRENT_SWEEP_STATE.EnableSweeping()
	fmt.Fprintf(w, "enabled")
}
func autoSweepPeersDisable(w http.ResponseWriter, r *http.Request) {
	log.Printf("disabling sweeping")
	GLOBAL_CURRENT_SWEEP_STATE.DisableSweeping()
	fmt.Fprintf(w, "disabled")
}

func fastPeerage(peers *string) {
	log.Info("Peers file provided...reading", zap.String("filename", *peers))
	filedata, err := ioutil.ReadFile(*peers)
	if err != nil {
		log.Error("Unable to read peer file", zap.String("filename", *peers), zap.Error(err))
		return
	}
	peersStruct := &tiafapi.Peerage{}
	if err := json.Unmarshal(filedata, peersStruct); err != nil {
		log.Error("unable to decode peer file", zap.String("filename", *peers), zap.Error(err))
		return
	}

	GLOBAL_PEERS.Lock()
	GLOBAL_PEERS.Peers = peersStruct.Peers
	GLOBAL_PEERS.Unlock()

	GLOBAL_CURRENT_SWEEP_STATE.EnableSweeping()
}

var NODE_MEMPOOL *tiaf.Fifo

type InternalTrie struct {
	*trie.Trie
	sync.RWMutex
}

func (t *InternalTrie) Put(a []byte) {
	t.Lock()
	defer t.Unlock()
	t.Trie.Put(string(a))
}

func (t *InternalTrie) Exist(a []byte) bool {
	t.RLock()
	defer t.RUnlock()
	return t.Trie.Exist(string(a))
}

var MEMPOOL_CHECKER *InternalTrie

const DURANCE = time.Second * 30

// Allow for spikes
const MAX_LOCAL_POOL = 1000

func processRecords() {
	// time before we startup...
	time.Sleep(time.Second * 1)

	nextDump := time.Now().Add(DURANCE)
	log.Info("record processor...", zap.Duration("time between flushes", DURANCE),
		zap.Int("max local pool size", MAX_LOCAL_POOL))
	// never ending loop

	for {
		// [100, 10000)
		sleepTime := 500 + time.Duration(rand.Intn(10000))

		if time.Now().After(nextDump) || NODE_MEMPOOL.Length() >= MAX_LOCAL_POOL {

			buffer := tiafapi.RecordCollection{Items: []tiafapi.Record{}}
			NODE_MEMPOOL.Lock()
			for {
				res, err := NODE_MEMPOOL.Pop()
				if err != nil {
					log.Warn("failed to pop from the mem pool")
					continue
				}
				if res == nil {
					break
				}

				seen, err := GLOBAL_CHAIN.HasSeen(*res)
				if err != nil {
					log.Warn("error in viewing record")
				}
				if seen {
					log.Info("transaction seen in chain")
				}
				buffer.Items = append(buffer.Items, *res)
			}
			if NODE_MEMPOOL.Length() != 0 {
				log.Error("Consistency error")
			}

			NODE_MEMPOOL.Unlock()

			if len(buffer.Items) > 0 {
				log.Info("hammering into the chain now")
				err := GLOBAL_CHAIN.AppendBlock(buffer)
				if err != nil {
					log.Warn("Inability to append records", zap.Error(err))
				}
			}
			nextDump = time.Now().Add(DURANCE)
		} else {
			log.Info("statistics...", zap.Duration("time until next run", nextDump.Sub(time.Now())), zap.Int("headroom in pool", MAX_LOCAL_POOL-NODE_MEMPOOL.Length()))
		}

		d := time.Millisecond * sleepTime
		log.Printf(" record processor sleeping %v", d)
		time.Sleep(d)
	}
}

// enterRecord receives data in the body, hashes it, and enters it in the FIFO mempool queue.
func enterRecord(w http.ResponseWriter, r *http.Request) {
	decoder := json.NewDecoder(r.Body)

	input := tiafapi.Record{}
	if err := decoder.Decode(&input); err != nil {
		w.WriteHeader(http.StatusBadRequest)
		_, _ = w.Write([]byte("couldn't decode"))

		return
	}
	log.Info("information", zap.Any("o", input), zap.String("sender", r.Host))

	// these 3 need to be in a commono mutex for this operation.
	seen, _ := GLOBAL_CHAIN.HasSeen(input)

	if MEMPOOL_CHECKER.Exist(input.GetHash()) || seen {
		w.WriteHeader(http.StatusNotAcceptable)
		log.Info("Attempted double-send of record")
		return
	}

	log.Info("Preparing to store record...")
	NODE_MEMPOOL.Lock()
	err := NODE_MEMPOOL.Put(&input)
	defer NODE_MEMPOOL.Unlock()
	if err != nil {
		log.Error("Failure storing the record in the FIFO", zap.Error(err))
		w.WriteHeader(http.StatusInternalServerError)
		_, _ = w.Write([]byte("error"))
		return
	} else {
		MEMPOOL_CHECKER.Put(input.GetHash())

		log.Info("record logged into the mempool")

		peer_list := GLOBAL_PEERS.GetPeers()
		// fan out into go routines
		go func() {
			for _, peer := range peer_list {
				p := peer

				u, err := url.Parse(p)
				if err != nil {
					log.Info("unable to send to peer, unparsable url", zap.Error(err))
					continue
				}
				if u.Host == r.RemoteAddr {
					log.Info("refusing to echo back to sender")
					continue
				}

				log.Info("writing record to peer", zap.String("host", p), zap.Any("req", r.URL))
				err = tiafapi.PutRecord(&input, p)
				if err != nil {
					log.Warn("unable to put record to peer", zap.String("host", p))
				}
			}
		}()
		_, _ = w.Write([]byte("ok"))
		return
	}
}

type Statistics struct {
	MempoolSize int
	ChainSize   int
}

func statistics(w http.ResponseWriter, r *http.Request) {
	bytes, err := json.Marshal(&Statistics{
		MempoolSize: NODE_MEMPOOL.Length(),
		ChainSize:   GLOBAL_CHAIN.Length(),
	})
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		fmt.Fprintf(w, "failed to gether stats")
	}

	_, _ = w.Write(bytes)
}

//////////////////////////////////////////////////////////////
func init() {
	GLOBAL_CHAIN = tiaf.NewChain()
	GLOBAL_PEERS = tiaf.NewPeers()
	MEMPOOL_CHECKER = &InternalTrie{
		Trie: trie.New(nil),
	}
	rand.Seed(time.Now().UnixNano())

	NODE_MEMPOOL = tiaf.NewFifo()
	GLOBAL_CURRENT_SWEEP_STATE = &doSweep{state: WillNotSweep}
}

func Default(w http.ResponseWriter, r *http.Request) {

	w.WriteHeader(http.StatusOK)

	fmt.Fprintf(w, "ok")
}

func Index(w http.ResponseWriter, r *http.Request) {
	index := `<html>
   <head>
      <script type = "text/javascript">
			function setChainDiv(data) {
				console.log(data);
                let pp = JSON.stringify(data["block_list"],null,8);
				document.getElementById("chain").innerHTML=pp;
			}
             function viewChain() {

				fetch('/api/chain')
				.then(response => response.json())
				.then(setChainDiv);
            }
      </script>
   </head>

   <body>
<h1> tiaf</h1>
      <input type = "button" onclick = "viewChain()" value = "ViewChain" />
		<pre><div  id="chain"></div></pre>

<hr>

   </body>
</html>`
	fmt.Fprintf(w, index)

	w.WriteHeader(http.StatusOK)
}

func Wut(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNotFound)
	fmt.Fprintf(w, "your content is in another url")
}

func loggerHandler(h http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()
		h.ServeHTTP(w, r)
		log.Printf("%s %s %v", r.Method, r.URL.Path, time.Since(start))
	})
}

//////////////////////////////////////////////////////////////
func main() {
	parser := argparse.NewParser("print", "runs tiaf node")

	host := parser.String("i", "ip", &argparse.Options{Required: false, Help: "ip to bind to", Default: "0.0.0.0"})
	port := parser.String("p", "port", &argparse.Options{Required: false, Help: "port to bind to", Default: "1337"})
	peers := parser.String("q", "peers", &argparse.Options{Required: false, Help: "file containing name of peers; if provided, autosweeps immediately"})
	// Parse input
	err := parser.Parse(os.Args)
	if err != nil {
		// In case of error print error and print usage
		// This can also be done by passing -h or --help flags
		fmt.Print(parser.Usage(err))
		return
	}

	log.Printf("Good morning, Bilbo Baggins. I am listening on %s:%s", *host, *port)

	r := mux.NewRouter()
	errorChain := alice.New(loggerHandler)
	r.HandleFunc("/", Index)
	r.HandleFunc("/healthz", Default)
	r.HandleFunc("/api/chain", returnChain).Methods("GET")
	r.HandleFunc("/api/chain", evaluateNewChainForAcceptance).Methods("PUT")
	r.HandleFunc("/api/chain/compare", measureOtherChain).Methods("POST")

	r.HandleFunc("/api/block", appendBlock).Methods("PUT")

	r.HandleFunc("/api/record", enterRecord).Methods("PUT")
	r.HandleFunc("/api/statistics", statistics).Methods("GET")

	r.HandleFunc("/api/peers", putPeers).Methods("PUT")
	r.HandleFunc("/api/peers", getPeers).Methods("GET")
	r.HandleFunc("/api/peers/sweep", sweepPeers).Methods("POST")
	r.HandleFunc("/api/peers/sweep/auto", autoSweepPeersEnable).Methods("PUT")
	r.HandleFunc("/api/peers/sweep/auto", autoSweepPeersDisable).Methods("DELETE")

	r.NotFoundHandler = http.HandlerFunc(Wut)

	if *peers != "" {
		fastPeerage(peers)
	}

	go sweeperDaemon()
	go processRecords()

	srv := &http.Server{
		Handler:      errorChain.Then(r),
		Addr:         fmt.Sprintf("%s:%s", *host, *port),
		WriteTimeout: 15 * time.Second,
		ReadTimeout:  15 * time.Second,
	}

	log.Fatal("server failure", zap.Error(srv.ListenAndServe()))
}

func sweeperDaemon() {
	// time before we startup...
	time.Sleep(time.Second * 1)
	// never ending loop
	for {
		// [3, 20)
		sleepTime := 3 + time.Duration(rand.Intn(17))
		if GLOBAL_CURRENT_SWEEP_STATE.IsSweeping() {
			log.Printf("autosweeper beginning sweep")
			internalSweeper()
			log.Printf(" autosweeper sleeping %d seconds", sleepTime)
		}

		d := time.Second * sleepTime
		time.Sleep(d)
	}
}
