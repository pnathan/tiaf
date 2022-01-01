package tiaf

import (
	"sync"

	"gitlab.com/pnathan/tiaf/src/lib/tiafapi"
)

type InternalPeers struct {
	tiafapi.Peerage
	sync.Mutex
}

func NewPeers() *InternalPeers {
	p := tiafapi.Peerage{Peers: []string{}}
	return &InternalPeers{Peerage: p}
}

func (r *InternalPeers) GetPeers() []string {
	r.Lock()
	defer r.Unlock()
	retval := []string{}
	for _, e := range r.Peers {
		retval = append(retval, e)
	}
	return retval
}
