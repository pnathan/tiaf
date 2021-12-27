package main

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"sync"
	"time"
	"github.com/akamensky/argparse"

	"github.com/gorilla/mux"
	"github.com/justinas/alice"

	"gitlab.com/pnathan/studchain/src/lib/studchain"
)

var Chain *studchain.Chain

func returnChain(w http.ResponseWriter, r *http.Request) {

	bytes, err := json.Marshal(Chain)
	if err != nil {
		log.Printf("error: %v", err)
		w.WriteHeader(http.StatusInternalServerError)
		return
	}
	fmt.Fprint(w, string(bytes))

	w.WriteHeader(http.StatusOK)
}

func appendBlock(w http.ResponseWriter, r *http.Request) {
	if err := r.ParseForm(); err != nil {
		_, _ = w.Write([]byte("query parse fail"))
		w.WriteHeader(400)
		return
	}
	data := r.Form["data"][0]
	log.Printf("data: %v", data)
	if data == "" {
		_, _ = w.Write([]byte("empty data"))
		w.WriteHeader(999)
		return
	}

	Chain.AppendBlock([]byte(data))
	_, _ = w.Write([]byte("ok"))
	w.WriteHeader(http.StatusOK)
}

func appendChain(w http.ResponseWriter, r *http.Request) {
	decoder := json.NewDecoder(r.Body)

	candidateChain := studchain.Chain{}
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
	if Chain.Length() < candidateChain.Length() {
		Chain.Mutex.Lock()
		defer Chain.Mutex.Unlock()
		Chain.List = candidateChain.List
		_, _ = w.Write([]byte("replaced with new chain"))
		w.WriteHeader(http.StatusCreated)
	}
	w.WriteHeader(http.StatusOK)
}

func measureOtherChain(w http.ResponseWriter, r *http.Request) {
	decoder := json.NewDecoder(r.Body)

	candidateChain := studchain.Chain{}
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

	if Chain.Length() < candidateChain.Length() {
		_, _ = w.Write([]byte("submitted chain is newer"))
	} else {
		_, _ = w.Write([]byte("contained chain is at least equal, if not newer"))
	}

	w.WriteHeader(http.StatusOK)
}

type Peerage struct {
	Peers []string `json:"peers"`
	sync.Mutex `json:"-"`
}

var Peers Peerage

func setPeers(w http.ResponseWriter, r *http.Request) {
	decoder := json.NewDecoder(r.Body)

	peers := Peerage{}
	if err := decoder.Decode(&peers); err != nil {
		_, _ = w.Write([]byte("couldn't decode"))
		w.WriteHeader(http.StatusBadRequest)
		return
	}

	Peers.Lock()
	defer Peers.Unlock()
	Peers.Peers = peers.Peers


	_, _ = w.Write([]byte("ok"))
	w.WriteHeader(http.StatusOK)
}

func getPeers(w http.ResponseWriter, r *http.Request) {
	Peers.Lock()
	defer Peers.Unlock()
	text, err := json.Marshal(Peers)
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
	Peers.Lock()
	defer Peers.Unlock()
	endpoints := Peers.Peers
	for _, addr := range endpoints {
		formulatedAddress := fmt.Sprintf("%v/api/chain/get", addr)
		log.Printf("Sweeping peer %v", formulatedAddress)

		resp, err := http.Get(formulatedAddress)
		if err != nil {
			log.Printf("error sweeping peer %v", err)
			continue
		}
		defer resp.Body.Close()


		decoder := json.NewDecoder(resp.Body)

		candidateChain := studchain.Chain{}
		if err := decoder.Decode(&candidateChain); err != nil {
			log.Printf("%#v", err)
			fmt.Fprintf(w, "couldn't decode %v", formulatedAddress)
			continue
		}

		if !candidateChain.ValidateChain() {
			fmt.Fprintf(w, "invalid chain gotten %v", formulatedAddress)
			continue
		}

		if Chain.Length() < candidateChain.Length() {
			Chain.Mutex.Lock()
			defer Chain.Mutex.Unlock()
			Chain.List = candidateChain.List
			fmt.Fprintf(w, "updated from %v", formulatedAddress)
		}
	}
}

//////////////////////////////////////////////////////////////
func init() {
	Chain = &studchain.Chain{
		List: []studchain.Block{*studchain.Genesis()},
	}
	Peers = Peerage{Peers: []string{}}
}


func Default(w http.ResponseWriter, r *http.Request) {

	w.WriteHeader(http.StatusOK)

	fmt.Fprintf(w, "ok")
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
	parser := argparse.NewParser("print", "runs studcoin node")

	host := parser.String("i", "ip", &argparse.Options{Required: false, Help: "ip to bind to", Default: "127.0.0.1"})
	port := parser.String("p", "port", &argparse.Options{Required: false, Help: "port to bind to", Default: "1337"})
	// Parse input
	err := parser.Parse(os.Args)
	if err != nil {
		// In case of error print error and print usage
		// This can also be done by passing -h or --help flags
		fmt.Print(parser.Usage(err))
		return
	}

	fmt.Println("good morning")
	r := mux.NewRouter()
	errorChain := alice.New(loggerHandler)
	r.HandleFunc("/", Default)
	r.HandleFunc("/api/chain/get", returnChain)
	r.HandleFunc("/api/chain/append", appendChain)
	r.HandleFunc("/api/chain/compare", measureOtherChain)

	r.HandleFunc("/api/block/append", appendBlock)

	r.HandleFunc("/api/peers/set", setPeers)
	r.HandleFunc("/api/peers/get", getPeers)
	r.HandleFunc("/api/peers/sweep", sweepPeers)

	r.NotFoundHandler = http.HandlerFunc(Wut)

	srv := &http.Server{
		Handler:      errorChain.Then(r),
		Addr:         fmt.Sprintf("%s:%s", *host, *port),
		WriteTimeout: 15 * time.Second,
		ReadTimeout:  15 * time.Second,
	}

	log.Fatal(srv.ListenAndServe())
}
