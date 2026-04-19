// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "AssistantIntents",
    platforms: [
        .iOS(.v16),
        .macOS(.v13),
    ],
    products: [
        .library(
            name: "AssistantIntents",
            targets: ["AssistantIntents"]
        ),
    ],
    targets: [
        .target(
            name: "AssistantIntents",
            path: "Sources/AssistantIntents"
        ),
        .testTarget(
            name: "AssistantIntentsTests",
            dependencies: ["AssistantIntents"],
            path: "Tests/AssistantIntentsTests"
        ),
    ]
)
