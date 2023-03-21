terraform {
  required_providers {
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "2.18.1"
    }
    helm = {
      source  = "hashicorp/helm"
      version = "2.9.0"
    }
  }
}

locals {
  kubeconfig = yamldecode(base64decode(data.terraform_remote_state.scuffle_infra.outputs.kubeconfig))
}


provider "kubernetes" {
  host                   = local.kubeconfig.clusters[0].cluster.server
  cluster_ca_certificate = base64decode(local.kubeconfig.clusters[0].cluster.certificate-authority-data)
  token                  = local.kubeconfig.users[0].user.token
}

provider "helm" {
  kubernetes {
    host                   = local.kubeconfig.clusters[0].cluster.server
    cluster_ca_certificate = base64decode(local.kubeconfig.clusters[0].cluster.certificate-authority-data)
    token                  = local.kubeconfig.users[0].user.token
  }
}
