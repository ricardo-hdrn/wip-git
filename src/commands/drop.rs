use crate::git;
use crate::ref_name;

pub struct DropResult {
    pub name: String,
    pub wip_ref: String,
}

pub fn run(name: String, remote: String) -> Result<DropResult, String> {
    let user = ref_name::user()?;
    let name = ref_name::resolve_name(Some(name))?;
    let wip_ref = ref_name::wip_ref(&name, &user);

    // Delete remote ref by pushing empty refspec
    let delete_refspec = format!(":{wip_ref}");
    git::git(&["push", &remote, &delete_refspec])?;

    Ok(DropResult { name, wip_ref })
}
