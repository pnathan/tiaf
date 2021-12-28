package main

import (
	"encoding/json"
	"fmt"
	"os"

	appsv1 "github.com/pulumi/pulumi-kubernetes/sdk/v3/go/kubernetes/apps/v1"
	corev1 "github.com/pulumi/pulumi-kubernetes/sdk/v3/go/kubernetes/core/v1"
	metav1 "github.com/pulumi/pulumi-kubernetes/sdk/v3/go/kubernetes/meta/v1"
	"github.com/pulumi/pulumi/sdk/v3/go/pulumi"
)

type Peerage struct {
	Peers []string `json:"peers"`
}

func main() {

	deploymentName := "tiaf"
	namespace := deploymentName
	version := os.Getenv("TIAF_VERSION")
	pulumi.Run(func(ctx *pulumi.Context) error {

		appLabels := pulumi.StringMap{
			"app":     pulumi.String(deploymentName),
			"version": pulumi.String(version),
		}

		md := &metav1.ObjectMetaArgs{
			Labels:    appLabels,
			Namespace: pulumi.StringPtr(namespace),
			Name:      pulumi.StringPtr(deploymentName),
		}

		dat := Peerage{
			Peers: []string{
				"http://tiaf-0.tiaf.tiaf:1337",
				"http://tiaf-1.tiaf.tiaf:1337",
				"http://tiaf-2.tiaf.tiaf:1337",
			},
		}

		configData, err := json.Marshal(dat)

		tiafConfig, err := corev1.NewConfigMap(ctx, deploymentName, &corev1.ConfigMapArgs{
			Metadata: &metav1.ObjectMetaArgs{
				Labels:    appLabels,
				Name:      pulumi.StringPtr(deploymentName),
				Namespace: pulumi.String(namespace),
			},
			Data: pulumi.StringMap{"peers.json": pulumi.String(string(configData))},
		})

		tiafConfigName := tiafConfig.Metadata.Name()

		svc, err := corev1.NewService(ctx, deploymentName, &corev1.ServiceArgs{
			Metadata: md,
			Spec: corev1.ServiceSpecArgs{
				ClusterIP: pulumi.StringPtr("None"),
				Ports: corev1.ServicePortArray{
					corev1.ServicePortArgs{
						TargetPort: pulumi.Int(1337),
						Port:       pulumi.Int(80),
					},
				},
				Selector: appLabels,
			},
		},
		)

		ctx.Export("ss name", svc.Metadata.Elem().Name())

		selector := &metav1.LabelSelectorArgs{
			MatchLabels: appLabels,
		}
		tiafConfigVolumeName := pulumi.String("tiaf-configs")

		ss, err := appsv1.NewStatefulSet(ctx, deploymentName, &appsv1.StatefulSetArgs{
			Metadata: md,
			Spec: appsv1.StatefulSetSpecArgs{
				// enough time to replicate data.
				MinReadySeconds:     pulumi.Int(30),
				PodManagementPolicy: pulumi.StringPtr("OrderedReady"),
				Replicas:            pulumi.Int(3),
				Selector:            selector,
				ServiceName:         pulumi.String(deploymentName),
				Template: &corev1.PodTemplateSpecArgs{
					Metadata: &metav1.ObjectMetaArgs{
						Labels: appLabels,
					},
					Spec: &corev1.PodSpecArgs{
						Containers: corev1.ContainerArray{
							corev1.ContainerArgs{
								Name: pulumi.String("tiaf"),
								Args: pulumi.StringArray{
									pulumi.String("/tiaf"), pulumi.String("-q"), pulumi.String("/etc/tiaf/peers.json"),
								},
								ImagePullPolicy: pulumi.String("Always"),
								Image:           pulumi.String(fmt.Sprintf("gcr.io/sapient-fabric-207305/tiaf:%s", version)),
								Ports: corev1.ContainerPortArray{
									corev1.ContainerPortArgs{
										ContainerPort: pulumi.Int(1337),
									},
								},
								VolumeMounts: &corev1.VolumeMountArray{
									&corev1.VolumeMountArgs{
										Name: tiafConfigVolumeName,

										MountPath: pulumi.String("/etc/tiaf/"),
									},
								},
							},
						},
						Volumes: &corev1.VolumeArray{
							&corev1.VolumeArgs{
								Name: tiafConfigVolumeName,
								ConfigMap: &corev1.ConfigMapVolumeSourceArgs{
									Name: tiafConfigName,
								},
							},
						},
					},

					//UpdateStrategy:       nil,
				},
			},
		})

		if err != nil {
			return err
		}

		ctx.Export("ss name", ss.Metadata.Elem().Name())

		return nil
	})
}
