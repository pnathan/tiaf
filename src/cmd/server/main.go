package main

import (
	"encoding/json"
	"fmt"
	"io/ioutil"
	"math/rand"
	"net/http"
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
		_, _ = w.Write([]byte("query parse fail"))
		w.WriteHeader(http.StatusBadRequest)
		return
	}
	log.Printf("data: %v", data)
	if string(data.Data) == "" {
		_, _ = w.Write([]byte("empty data!"))
		w.WriteHeader(http.StatusNotAcceptable)
		return
	}

	if err := GLOBAL_CHAIN.AppendBlock(tiafapi.Datatype(data.Data)); err != nil {
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
		_, _ = w.Write([]byte("couldn't decode"))
		w.WriteHeader(http.StatusBadRequest)
		return
	}

	if !candidateChain.ValidateChain() {
		_, _ = w.Write([]byte("invalid chain submitted"))
		w.WriteHeader(http.StatusBadRequest)
		return
	}

	// this is actually very silly!
	// https://lhartikk.github.io/jekyll/update/2017/07/14/chapter1.html
	// Really? Longest chain?
	if GLOBAL_CHAIN.Length() < candidateChain.Length() {
		GLOBAL_CHAIN.Mutex.Lock()
		defer GLOBAL_CHAIN.Mutex.Unlock()
		GLOBAL_CHAIN.BlockList = candidateChain.BlockList
		_, _ = w.Write([]byte("replaced with new chain"))
		w.WriteHeader(http.StatusCreated)
	} else {
		_, _ = w.Write([]byte("no change"))
	}
}

func measureOtherChain(w http.ResponseWriter, r *http.Request) {
	decoder := json.NewDecoder(r.Body)

	candidateChain := tiafapi.Chain{}
	if err := decoder.Decode(&candidateChain); err != nil {
		log.Printf("%#v", err)
		_, _ = w.Write([]byte("couldn't decode"))
		w.WriteHeader(http.StatusBadRequest)
		return
	}

	if !candidateChain.ValidateChain() {
		_, _ = w.Write([]byte("invalid chain submitted"))
		w.WriteHeader(http.StatusBadRequest)
		return
	}

	if GLOBAL_CHAIN.Length() < candidateChain.Length() {
		_, _ = w.Write([]byte("submitted chain is newer"))
	} else {
		_, _ = w.Write([]byte("contained chain is at least equal, if not newer"))
	}
}

type InternalPeers struct {
	tiafapi.Peerage
	sync.Mutex
}

var GLOBAL_PEERS InternalPeers

func putPeers(w http.ResponseWriter, r *http.Request) {
	decoder := json.NewDecoder(r.Body)

	peers := tiafapi.Peerage{}
	if err := decoder.Decode(&peers); err != nil {
		_, _ = w.Write([]byte("couldn't decode"))
		w.WriteHeader(http.StatusBadRequest)
		return
	}

	GLOBAL_PEERS.Lock()
	defer GLOBAL_PEERS.Unlock()
	GLOBAL_PEERS.Peers = peers.Peers

	_, _ = w.Write([]byte("ok"))
}

func getPeers(w http.ResponseWriter, r *http.Request) {
	GLOBAL_PEERS.Lock()
	defer GLOBAL_PEERS.Unlock()
	text, err := json.Marshal(GLOBAL_PEERS.Peers)
	if err != nil {
		w.WriteHeader(http.StatusBadRequest)
		_, _ = w.Write([]byte("err"))
		return
	}
	_, _ = w.Write(text)
	w.WriteHeader(http.StatusOK)
}

// sweepPeers is a pull based chain getter.
func sweepPeers(w http.ResponseWriter, r *http.Request) {
	internalSweeper()
}

func internalSweeper() []string {
	GLOBAL_PEERS.Lock()
	defer GLOBAL_PEERS.Unlock()
	endpoints := GLOBAL_PEERS.Peers
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
		GLOBAL_CHAIN.Mutex.Lock()
		defer GLOBAL_CHAIN.Mutex.Unlock()
		GLOBAL_CHAIN.BlockList = candidateChain.BlockList
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

//////////////////////////////////////////////////////////////
func init() {
	GLOBAL_CHAIN = &tiaf.InternalChain{
		Chain: tiafapi.Chain{BlockList: []tiafapi.Block{*tiafapi.Genesis()}},
	}
	p := tiafapi.Peerage{Peers: []string{}}
	GLOBAL_PEERS = InternalPeers{Peerage: p}
	rand.Seed(time.Now().UnixNano())

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

	srv := &http.Server{
		Handler:      errorChain.Then(r),
		Addr:         fmt.Sprintf("%s:%s", *host, *port),
		WriteTimeout: 15 * time.Second,
		ReadTimeout:  15 * time.Second,
	}

	log.Fatal("server failure", zap.Error(srv.ListenAndServe()))
}

func sweeperDaemon() {
	// 10 seconds before we startup...
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
