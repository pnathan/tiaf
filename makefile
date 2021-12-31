all: tiaf tiaf-client

docker: .dockerbuild

VERSION := v7

.dockerbuild: Dockerfile tiaf makefile
	docker build . -t gcr.io/sapient-fabric-207305/tiaf:$(VERSION)
	docker push gcr.io/sapient-fabric-207305/tiaf:$(VERSION)
	docker tag gcr.io/sapient-fabric-207305/tiaf:$(VERSION) tiaf:$(VERSION)
	touch .dockerbuild

minikube: .minikube
.minikube: .dockerbuild k8s/system.yaml .k8sss
	minikube image load tiaf:$(VERSION)
	kubectl config use-context minikube
	kubectl apply -f k8s/system.yaml
	touch .minikube


.k8sss: k8s/statefulset.yaml
	kubectl apply -f k8s/statefulset.yaml
	touch .k8sss

pulumi:
	cd k8s/tiaf-deployment/ && TIAF_VERSION=$(VERSION) pulumi up -y --color never

RESOURCES := $(shell find src/ -name *.go -print)

tiaf-client:  $(RESOURCES)
	CGO_ENABLED=0 go1.18beta1 build -o tiaf-client src/cmd/client/main.go
tiaf:  $(RESOURCES)
	CGO_ENABLED=0 go1.18beta1 build -o tiaf src/cmd/server/main.go
