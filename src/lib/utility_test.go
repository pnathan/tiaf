package lib_test
import (
	"fmt"
	"testing"

	"gitlab.com/pnathan/studchain/src/lib"
)


func TestCaseInt(t *testing.T) {
	b1 := []int{1,2}
	b2 := []int{3,4}
	b3 := lib.Concat(b1, b2)
	fmt.Printf("%v", b3)
}


func TestCaseByte(t *testing.T) {
	b1 := []byte{1,2}
	b2 := []byte{3,4}
	b3 := lib.Concat(b1, b2)
	fmt.Printf("%v", b3)
}