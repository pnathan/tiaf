package tiaf

import (
	"reflect"
	"testing"

	"github.com/google/uuid"

	"gitlab.com/pnathan/tiaf/src/lib/tiafapi"
)

func TestFifo_Pop(t *testing.T) {
	stdUUid := uuid.New()

	tests := []struct {
		name    string
		r       []*tiafapi.Record
		want    *tiafapi.Record
		wantErr bool
	}{
		{
			name: "one",
			r: []*tiafapi.Record{{
				Uuid:      stdUUid,
				Timestamp: 0,
				Entry:     "data",
			}},
			want: &tiafapi.Record{
				Uuid:      stdUUid,
				Timestamp: 0,
				Entry:     "data",
			},
		},
		{
			name: "two",
			r: []*tiafapi.Record{
				{
					Uuid:      stdUUid,
					Timestamp: 0,
					Entry:     "data",
				},
				{
					Uuid:      stdUUid,
					Timestamp: 1000,
					Entry:     "BOB BOB BOB",
				},
			},
			want: &tiafapi.Record{
				Uuid:      stdUUid,
				Timestamp: 0,
				Entry:     "data",
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			f := NewFifo()
			for _, r := range tt.r {
				f.Put(r)
			}
			got, err := f.Pop()
			if (err != nil) != tt.wantErr {
				t.Errorf("Pop() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("Pop() got = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestPrecise(t *testing.T) {
	stdUUid := uuid.New()

	q := NewFifo()
	q.Put(&tiafapi.Record{
		Uuid:      stdUUid,
		Timestamp: 9,
		Entry:     "data",
	})
	q.Put(&tiafapi.Record{
		Uuid:      stdUUid,
		Timestamp: 10,
		Entry:     "DAT",
	})
	got, _ := q.Pop()
	if got.Timestamp != 9 {
		t.Errorf("failure")
	}

	got, _ = q.Pop()
	if got.Timestamp != 10 {
		t.Errorf("failure")
	}
	_, err := q.Pop()
	if err == nil {
		t.Errorf("this is bad")
	}

	// Test state changing.

	q.Put(&tiafapi.Record{
		Uuid:      stdUUid,
		Timestamp: 9,
		Entry:     "data",
	})
	q.Put(&tiafapi.Record{
		Uuid:      stdUUid,
		Timestamp: 10,
		Entry:     "DAT",
	})
	got, _ = q.Pop()
	if got.Timestamp != 9 {
		t.Errorf("failure")
	}

	got, _ = q.Pop()
	if got.Timestamp != 10 {
		t.Errorf("failure")
	}
	_, err = q.Pop()
	if err == nil {
		t.Errorf("this is bad")
	}

}
