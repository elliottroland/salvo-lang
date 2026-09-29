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
        String modelFile) {

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
                node.getStringMemberOrDefault("modelFile", "the pinned model"));
    }
}
