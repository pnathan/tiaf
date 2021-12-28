all: tiaf tiaf-client

docker: .dockerbuild

.dockerbuild: Dockerfile tiaf makefile
	docker build . -t gcr.io/sapient-fabric-207305/tiaf:v6
	docker push gcr.io/sapient-fabric-207305/tiaf:v6
	touch .dockerbuild

pulumi:
	cd k8s/tiaf-deployment/ && TIAF_VERSION=v6 pulumi up -y --color never

RESOURCES := $(shell find src/ -name *.go -print)

tiaf-client:  $(RESOURCES)
	CGO_ENABLED=0 go1.18beta1 build -o tiaf-client src/cmd/client/main.go
tiaf:  $(RESOURCES)
	CGO_ENABLED=0 go1.18beta1 build -o tiaf src/cmd/server/main.go
