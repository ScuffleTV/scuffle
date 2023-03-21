locals {
  postgres_host     = var.scuffle_infra.outputs.postgres_host
  postgres_username = "postgres"
  postgres_database = "scuffle-${trimprefix(terraform.workspace, "scuffle-app-")}"
}

resource "kubernetes_namespace" "api" {
  metadata {
    name = var.namespace
  }
}

module "migrations" {
  name              = "api-migrations"
  source            = "../migrations"
  docker_image      = "ghcr.io/scuffletv/migrator:latest"
  migrations_path   = "${path.module}/../../backend/migrations"
  namespace         = kubernetes_namespace.api.metadata[0].name
  tls_secret_name   = kubernetes_manifest.postgres_tls_auth.object.spec.secretName
  postgres_host     = local.postgres_host
  postgres_database = local.postgres_database
  postgres_username = local.postgres_username
}

resource "random_password" "jwt_secret" {
  length  = 32
  special = true
}

resource "kubernetes_secret" "scuffle_api" {
  metadata {
    name      = "scuffle-api"
    namespace = kubernetes_namespace.api.metadata[0].name
  }
  data = {
    "config.yaml" = templatefile("${path.module}/config.yaml", {
      database_url         = "postgres://${local.postgres_username}@${local.postgres_host}/${local.postgres_database}?sslmode=verify-full&sslrootcert=/var/run/secrets/postgres/ca.crt&sslcert=/var/run/secrets/postgres/tls.crt&sslkey=/var/run/secrets/postgres/tls.key"
      turnstile_secret_key = var.turnstile_secret_key
      jwt_issuer           = "https://api.scuffle.tv"
      jwt_secret           = random_password.jwt_secret.result
    })
  }
}

resource "kubernetes_manifest" "postgres_tls_auth" {
  manifest = {
    apiVersion = "cert-manager.io/v1"
    kind       = "Certificate"
    metadata = {
      name      = "api-postgres-tls-auth"
      namespace = kubernetes_namespace.api.metadata[0].name
    }
    spec = {
      secretName = "api-postgres-tls-auth"
      # this is the name of the user to authenticate as
      commonName = "postgres"
      privateKey = {
        rotationPolicy = "Always"
        algorithm      = "ECDSA"
        size           = 256
        encoding       = "PKCS8"
      }
      usages      = ["client auth"]
      duration    = "360h0m0s" # 15 days
      renewBefore = "180h0m0s" # 7 days
      subject = {
        organizations = ["scuffle"]
      }
      issuerRef = {
        name = "postgres-ca"
        kind = "ClusterIssuer"
      }
    }
  }
}

resource "kubernetes_deployment" "api" {
  metadata {
    name = "scuffle-api"
    labels = {
      app = "scuffle-api"
    }
    namespace = kubernetes_namespace.api.metadata[0].name
  }

  timeouts {
    create = "2m"
    update = "2m"
    delete = "2m"
  }

  spec {
    selector {
      match_labels = {
        app = "scuffle-api"
      }
    }
    template {
      metadata {
        labels = {
          app = "scuffle-api"
        }
      }
      spec {
        container {
          name              = "scuffle-api"
          image             = var.docker_image
          image_pull_policy = "Always"
          port {
            container_port = 8080
            name           = "http"
          }
          env {
            name  = "SCUF_CONFIG_FILE"
            value = "/app/config.yaml"
          }
          readiness_probe {
            initial_delay_seconds = 5
            period_seconds        = 5
            http_get {
              path = "/v1/health"
              port = 8080
            }
          }
          liveness_probe {
            initial_delay_seconds = 5
            period_seconds        = 5
            http_get {
              path = "/v1/health"
              port = 8080
            }
          }
          startup_probe {
            initial_delay_seconds = 5
            period_seconds        = 5
            http_get {
              path = "/v1/health"
              port = 8080
            }
          }
          security_context {
            allow_privilege_escalation = false
            privileged                 = false
            read_only_root_filesystem  = true
            run_as_non_root            = true
            run_as_user                = 1000
            run_as_group               = 1000
            capabilities {
              drop = ["ALL"]
            }
          }
          resources {
            limits = {
              "cpu"    = "1000m"
              "memory" = "512Mi"
            }
            requests = {
              "cpu"    = "300m"
              "memory" = "512Mi"
            }
          }
          volume_mount {
            name       = "scuffle-api"
            mount_path = "/app/config.yaml"
            sub_path   = "config.yaml"
            read_only  = true
          }
          volume_mount {
            name       = "postgres-auth"
            mount_path = "/var/run/secrets/postgres"
            read_only  = true
          }
        }
        volume {
          name = "postgres-auth"
          secret {
            secret_name  = kubernetes_manifest.postgres_tls_auth.manifest.spec.secretName
            default_mode = "0644"
          }
        }
        volume {
          name = "scuffle-api"
          secret {
            secret_name  = kubernetes_secret.scuffle_api.metadata[0].name
            default_mode = "0644"
          }
        }
      }
    }
  }
}

resource "kubernetes_service" "api" {
  metadata {
    name      = "scuffle-api"
    namespace = kubernetes_namespace.api.metadata[0].name
  }

  spec {
    selector = {
      app = "scuffle-api"
    }
    port {
      name        = "http"
      port        = 8080
      target_port = "http"
    }
  }
}

resource "kubernetes_ingress_v1" "api" {
  metadata {
    name      = "scuffle-api"
    namespace = kubernetes_namespace.api.metadata[0].name
    annotations = {
      "external-dns.alpha.kubernetes.io/hostname" = "api.scuffle.tv"
      "kubernetes.io/ingress.class"               = "nginx"
      "cert-manager.io/cluster-issuer"            = "cloudflare"
    }
  }
  spec {
    rule {
      host = "api.scuffle.tv"
      http {
        path {
          path      = "/"
          path_type = "Prefix"
          backend {
            service {
              name = kubernetes_service.api.metadata[0].name
              port {
                name = kubernetes_service.api.spec[0].port[0].name
              }
            }
          }
        }
      }
    }
    tls {
      hosts       = ["api.scuffle.tv"]
      secret_name = "scuffle-api-tls"
    }
  }
}

resource "kubernetes_horizontal_pod_autoscaler" "api" {
  metadata {
    name      = "scuffle-api"
    namespace = kubernetes_namespace.api.metadata[0].name
  }

  spec {
    max_replicas = 10
    min_replicas = 1
    scale_target_ref {
      api_version = "apps/v1"
      kind        = "Deployment"
      name        = kubernetes_deployment.api.metadata[0].name
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
