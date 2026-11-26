// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "GraceRideKit",
    platforms: [
        .iOS(.v17),
        .macOS(.v14),
    ],
    products: [
        .library(
            name: "GraceRideKit",
            targets: ["GraceRideKit"]
        ),
    ],
    targets: [
        .target(
            name: "GraceRideKit",
            path: "Sources/GraceRideKit"
        ),
        .testTarget(
            name: "GraceRideKitTests",
            dependencies: ["GraceRideKit"],
            path: "Tests/GraceRideKitTests"
        ),
    ]
)
