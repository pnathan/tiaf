//go:build !go1.18
// +build !go1.18

package utility

import "encoding/binary"

func Concat(arrays ...[]byte) []byte {
	result := []byte{}
	for _, ele := range arrays {
		result = append(result, ele...)
	}
	return result
}

func UintToBytes(u uint64) []byte {
	int_buffer := make([]byte, binary.MaxVarintLen64)
	n := binary.PutUvarint(int_buffer, u)
	return int_buffer[:n]
}

func IntToBytes(u int64) []byte {
	int_buffer := make([]byte, binary.MaxVarintLen64)
	n := binary.PutVarint(int_buffer, u)
	return int_buffer[:n]
}
