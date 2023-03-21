resource "kubernetes_namespace" "website" {
  metadata {
    name = var.namespace
  }
}

resource "kubernetes_deployment" "website" {
  metadata {
    name = "scuffle-website"
    labels = {
      app = "scuffle-website"
    }
    namespace = kubernetes_namespace.website.metadata[0].name
  }

  timeouts {
    create = "2m"
    update = "2m"
    delete = "2m"
  }

  spec {
    selector {
      match_labels = {
        app = "scuffle-website"
      }
    }
    template {
      metadata {
        labels = {
          app = "scuffle-website"
        }
      }
      spec {
        container {
          name              = "scuffle-website"
          image             = var.docker_image
          image_pull_policy = "Always"
          port {
            container_port = 3000
            name           = "http"
          }
          env {
            name  = "PUBLIC_SSR_GQL_ENDPOINT"
            value = "http://${var.api_host}/v1/gql"
          }
          readiness_probe {
            initial_delay_seconds = 5
            period_seconds        = 5
            http_get {
              path = "/healthcheck"
              port = 3000
            }
          }
          liveness_probe {
            initial_delay_seconds = 5
            period_seconds        = 5
            http_get {
              path = "/healthcheck"
              port = 3000
            }
          }
          startup_probe {
            initial_delay_seconds = 5
            period_seconds        = 5
            http_get {
              path = "/healthcheck"
              port = 3000
            }
          }
          security_context {
            allow_privilege_escalation = false
            privileged                 = false
            read_only_root_filesystem  = false # deno needs to write to the filesystem
            run_as_non_root            = true
            run_as_user                = 1000
            run_as_group               = 1000
            capabilities {
              drop = ["ALL"]
            }
          }
          resources {
            limits = {
              "cpu"    = "300m"
              "memory" = "512Mi"
            }
            requests = {
              "cpu"    = "300m"
              "memory" = "512Mi"
            }
          }
        }
      }
    }
  }
}

resource "kubernetes_service" "website" {
  metadata {
    name      = "scuffle-website"
    namespace = kubernetes_namespace.website.metadata[0].name
  }

  spec {
    selector = {
      app = "scuffle-website"
    }
    port {
      name        = "http"
      port        = 3000
      target_port = "http"
    }
  }
}

resource "kubernetes_ingress_v1" "website" {
  metadata {
    name      = "scuffle-website"
    namespace = kubernetes_namespace.website.metadata[0].name
    annotations = {
      "external-dns.alpha.kubernetes.io/hostname" = "scuffle.tv"
      "kubernetes.io/ingress.class"               = "nginx"
      "cert-manager.io/cluster-issuer"            = "cloudflare"
    }
  }
  spec {
    rule {
      host = "scuffle.tv"
      http {
        path {
          path      = "/"
          path_type = "Prefix"
          backend {
            service {
              name = kubernetes_service.website.metadata[0].name
              port {
                name = kubernetes_service.website.spec[0].port[0].name
              }
            }
          }
        }
      }
    }
    tls {
      hosts       = ["scuffle.tv"]
      secret_name = "scuffle-website-tls"
    }
  }
}

resource "kubernetes_horizontal_pod_autoscaler" "website" {
  metadata {
    name      = "scuffle-website"
    namespace = kubernetes_namespace.website.metadata[0].name
  }

  spec {
    max_replicas = 5
    min_replicas = 1
    scale_target_ref {
      api_version = "apps/v1"
      kind        = "Deployment"
      name        = kubernetes_deployment.website.metadata[0].name
    }
    metric {
      type = "Resource"
      resource {
        name = "memory"
        target {
          type                = "Utilization"
          average_utilization = 75
        }
      }
    }
    metric {
      type = "Resource"
      resource {
        name = "cpu"
        target {
          type                = "Utilization"
          average_utilization = 75
        }
      }
    }
  }
}
