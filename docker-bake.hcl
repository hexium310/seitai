group "default" {
  targets = ["restarter", "seitai"]
}

target "restarter" {
  target = "restarter"
  tags = ["ghcr.io/hexium310/restarter"]
}

target "seitai" {
  target = "seitai"
  tags = ["ghcr.io/hexium310/seitai"]
}
