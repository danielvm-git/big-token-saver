// Fixture: Go greeter (bts-map snapshot test)
package main

import "fmt"

type Greeter struct{}

func (g Greeter) Greet(name string) string {
	return fmt.Sprintf("Hello, %s!", name)
}

func main() {
	g := Greeter{}
	fmt.Println(g.Greet("world"))
}
