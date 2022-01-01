package trie

type Trie struct {
	data map[byte]any
}

func New(ss []string) *Trie {
	result := &Trie{data: map[byte]any{}}
	for _, s := range ss {
		result.Put(s)
	}
	return result
}

func (t Trie) Exist(s string) bool {
	// definitionally either the empty string always exists or does not exist
	// here we define it as non existent.
	if s == "" {
		return false
	}
	return exists(t.data, []byte(s))
}

func (t Trie) Put(s string) {
	insert(t.data, []byte(s))
}

func insert(t map[byte]any, s []byte) {
	var temp map[byte]any
	// top level
	temp = t
	for _, c := range s {
		val, ok := temp[c]
		if !ok {
			temp[c] = make(map[byte]any)
			val = temp[c]
		}
		temp = val.(map[byte]any)
	}
}

func exists(t map[byte]any, s []byte) bool {
	temp := t
	for _, c := range s {
		val, ok := temp[c]
		if !ok {
			return false
		}
		temp = val.(map[byte]any)
	}
	return true
}

/*
type GenericTrie[T comparable] struct {
	data map[T]any
}

func (t GenericTrie) Put[T comparable](s []T) {
	var head T
	head = s[0]
	t.data[head] = 0
}
*/
