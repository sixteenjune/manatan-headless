// swift-tools-version: 6.2

import PackageDescription

let package = Package(
    name: "mangatan_ios",
    platforms: [.iOS(.v15)],
    targets: [
        .executableTarget(
            name: "mangatan_ios",
            path: "Sources"
        ),
    ]
)
