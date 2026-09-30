package salvo.codegen;

import java.util.ArrayList;
import java.util.List;

import software.amazon.smithy.model.node.Node;
import software.amazon.smithy.model.node.ObjectNode;
import software.amazon.smithy.model.shapes.ShapeId;

/** The plugin's settings (see {@link SalvoClientCodegen}). */
record Settings(
        ShapeId service,
        String module,
        String effect,
        List<String> operations,
        String rustCrate,
        String kotlinPackage,
        String modelFile,
        boolean forcePathStyle,
        List<String> omitMembers,
        java.util.Map<String, String> errorGroups) {

    static Settings from(ObjectNode node) {
        List<String> ops = new ArrayList<>();
        for (Node op : node.expectArrayMember("operations").getElements()) {
            ops.add(op.expectStringNode().getValue());
        }
        return new Settings(
                ShapeId.from(node.expectStringMember("service").getValue()),
                node.expectStringMember("module").getValue(),
                node.expectStringMember("effect").getValue(),
                ops,
                node.expectStringMember("rustCrate").getValue(),
                node.expectStringMember("kotlinPackage").getValue(),
                node.getStringMemberOrDefault("modelFile", "the pinned model"),
                // S3 only: with an `endpoint` override, address buckets in the
                // path (`http://host/bucket/key`) rather than the host name —
                // what a local stand-in or LocalStack needs.
                node.getBooleanMemberOrDefault("forcePathStyle", false),
                // Members to leave out, by shape id (`ns#Shape$Member`): where
                // the SDKs customize a member away from the model, so neither
                // spelling would convert.
                node.getArrayMember("omitMembers")
                        .map(a -> a.getElements().stream().map(e -> e.expectStringNode().getValue()).toList())
                        .orElse(List.of()),
                // Named groups of error codes, by the prefix of the model's
                // error names: `{"KmsErrorCode": "Kms"}` makes SQS's seven KMS
                // codes one literal union within the service's code union.
                node.getObjectMember("errorGroups")
                        .map(o -> {
                            java.util.Map<String, String> m = new java.util.LinkedHashMap<>();
                            o.getMembers().forEach((k, v) -> m.put(k.getValue(), v.expectStringNode().getValue()));
                            return m;
                        })
                        .orElse(java.util.Map.of()));
    }
}
