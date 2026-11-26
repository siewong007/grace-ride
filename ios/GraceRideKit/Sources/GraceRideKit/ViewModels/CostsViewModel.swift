import Foundation
import Observation

@MainActor
@Observable
public final class CostsViewModel {
    private let container: AppContainer

    public private(set) var loading: Bool = true
    public private(set) var refreshing: Bool = false
    public private(set) var offline: Bool = false
    public private(set) var error: String?
    public private(set) var shares: [Share] = []
    public private(set) var paying: Share?
    public var proofData: Data?
    public var proofFilename: String = "proof.jpg"
    public var proofMime: String = "image/jpeg"
    public var txnRef: String = ""
    public private(set) var submitting: Bool = false
    public private(set) var sheetError: String?
    public private(set) var sheetErrorKey: String?
    public private(set) var doneMessage: String?

    private var uploadedDigestKey: String?
    private var uploadedFileId: Int64?

    public init(container: AppContainer) {
        self.container = container
    }

    public func loadInitial() async {
        await load(first: true)
    }

    public func refresh() async {
        await load(first: false)
    }

    private func load(first: Bool) async {
        guard container.auth.me?.can("costs", act: "view") == true else {
            loading = false
            refreshing = false
            return
        }
        loading = first && shares.isEmpty
        refreshing = !first
        do {
            let list = try await container.shares.mine()
            loading = false
            refreshing = false
            shares = list
            error = nil
            offline = false
        } catch let e as ApiException {
            loading = false
            refreshing = false
            error = e.status == 0 ? nil : e.message
            offline = e.status == 0
        } catch {
            loading = false
            refreshing = false
            self.error = error.localizedDescription
        }
    }

    public func openPay(_ share: Share) {
        uploadedDigestKey = nil
        uploadedFileId = nil
        paying = share
        proofData = nil
        txnRef = share.txnRef
        sheetError = nil
        sheetErrorKey = nil
    }

    public func closePay() {
        uploadedDigestKey = nil
        uploadedFileId = nil
        paying = nil
        proofData = nil
        sheetError = nil
        sheetErrorKey = nil
    }

    public func setProof(data: Data?, filename: String = "proof.jpg", mime: String = "image/jpeg") {
        let newKey = data.map { "\($0.count)-\(filename)" }
        if newKey != uploadedDigestKey {
            uploadedDigestKey = nil
            uploadedFileId = nil
        }
        proofData = data
        proofFilename = filename
        proofMime = mime
        sheetError = nil
        sheetErrorKey = nil
    }

    public func setTxnRef(_ value: String) {
        txnRef = value
        sheetError = nil
        sheetErrorKey = nil
    }

    public func clearDone() { doneMessage = nil }

    public static func validateProofInput(hasPhoto: Bool, txnRef: String) -> String? {
        let trimmed = txnRef.trimmingCharacters(in: .whitespacesAndNewlines)
        if !hasPhoto { return "costs_err_missing_photo" }
        if trimmed.isEmpty { return "costs_err_missing_ref" }
        if trimmed.count < 4 { return "costs_err_ref_short" }
        return nil
    }

    public func submitProof() async {
        guard let share = paying else { return }
        guard container.auth.me?.can("costs", act: "update") == true else {
            sheetError = nil
            sheetErrorKey = "role_costs_access_required"
            return
        }
        if let validationKey = Self.validateProofInput(hasPhoto: proofData != nil, txnRef: txnRef) {
            sheetError = nil
            sheetErrorKey = validationKey
            return
        }
        guard let data = proofData else { return }
        guard data.count <= 8 * 1024 * 1024 else {
            sheetError = nil
            sheetErrorKey = "costs_err_file_too_large"
            return
        }
        guard !submitting else { return }
        submitting = true
        sheetError = nil
        sheetErrorKey = nil
        defer { submitting = false }

        let key = "\(data.count)-\(proofFilename)"
        do {
            let fileId: Int64
            if uploadedDigestKey == key, let cachedId = uploadedFileId {
                fileId = cachedId
            } else {
                let uploaded = try await container.shares.upload(data: data, filename: proofFilename, mime: proofMime)
                uploadedDigestKey = key
                uploadedFileId = uploaded.id
                fileId = uploaded.id
            }
            _ = try await container.shares.submitProof(
                shareId: share.id,
                fileId: fileId,
                txnRef: txnRef.trimmingCharacters(in: .whitespacesAndNewlines)
            )
            uploadedDigestKey = nil
            uploadedFileId = nil
            paying = nil
            proofData = nil
            doneMessage = "ok"
            await load(first: false)
        } catch let e as ApiException {
            sheetError = e.message
            sheetErrorKey = nil
        } catch {
            sheetError = error.localizedDescription
            sheetErrorKey = nil
        }
    }
}
