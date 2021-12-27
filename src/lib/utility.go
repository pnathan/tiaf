package lib

func Concat[T any](arrays...[]T) []T {
	result := []T{}
	for _, ele := range arrays {
		result = append(result, ele...)
	}
	return result
}
