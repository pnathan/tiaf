package utility_test

import (
	"fmt"
	"testing"

	"gitlab.com/pnathan/tiaf/src/lib/utility"
)

func TestCaseInt(t *testing.T) {
	b1 := []int{1, 2}
	b2 := []int{3, 4}
	b3 := utility.Concat(b1, b2)
	fmt.Printf("%v", b3)
}

func TestCaseByte(t *testing.T) {
	b1 := []byte{1, 2}
	b2 := []byte{3, 4}
	b3 := utility.Concat(b1, b2)
	fmt.Printf("%v", b3)
}
