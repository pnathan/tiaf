package trie

import (
	"testing"
)

func TestTrie(t *testing.T) {
	type args struct {
		t map[byte]any
		s string
	}
	tests := []struct {
		name string
		args args
	}{
		{
			name: "one",
			args: args{
				map[byte]any{},
				"x",
			},
		},
		{
			name: "two",
			args: args{
				map[byte]any{},
				"xo",
			},
		},
		{
			name: "three",
			args: args{
				map[byte]any{},
				"xox",
			},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if exists(tt.args.t, []byte(tt.args.s)) {
				t.Fatal("found inappropriately")
			}
			insert(tt.args.t, []byte(tt.args.s))
			if !exists(tt.args.t, []byte(tt.args.s)) {
				t.Fatal("could not find")
			}

			insert(tt.args.t, []byte(tt.args.s))
			if !exists(tt.args.t, []byte(tt.args.s)) {
				t.Fatal("could not find")
			}
		})
	}
}

func TestNew(t *testing.T) {
	tests := []struct {
		name string
		args []string
	}{
		{
			name: "misc",
			args: []string{"XoXoXo", "XUXoXo"},
		},
		{
			name: "misc more",
			args: []string{"XoXoX1", "XoXoX2"},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := New(tt.args)
			for _, e := range tt.args {
				if !got.Exist(e) {
					t.Errorf("%v: %v", e, got)
				}
			}
		})
	}
}

func TestExactCases(t *testing.T) {
	trie := New([]string{
		"XoXoX",
		"XoXoX1",
		"XoXoX2",
		"YoXoX",
	})
	if trie.Exist("") {
		t.Errorf("found empty string")
	}

	if trie.Exist("o") {
		t.Error("character wrongly installed")
	}

	if trie.Exist("XoXoX2-AND") {
		t.Error("too long string detecxted")
	}

	checks := []string{
		"X",
		"Y",
		"Xo",
		"Yo",
		"XoXoX",
		"XoXoX2",
	}
	for _, s := range checks {
		if !trie.Exist(s) {
			t.Error("character not installed")
		}
	}

}
